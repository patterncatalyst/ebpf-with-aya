---
title: "javagc"
order: 20
part: User-space & language probing
description: Time JVM garbage-collection pauses through the HotSpot USDT probes — learn what USDT is, why it beats uprobing a runtime's internals, and how to attach a uprobe at a USDT probe's offset when your toolkit has no first-class USDT helper.
duration: 30 minutes
---

This chapter times JVM **garbage-collection pauses** — and in doing so
introduces **USDT**, the mechanism language runtimes use to expose
stable internal events. The JVM, CPython, Node, and others ship USDT
probes precisely so tools like ours can observe them without reverse-
engineering their internals. `javagc` attaches to HotSpot's
`gc__begin`/`gc__end` and reports how long each pause lasts.

The code is in `examples/20-javagc/`. It's the most experimental chapter
in this part — read the verification notes. `./demo.sh` there builds,
deploys, and runs it; its `README.md` covers what it does and how to
drive it.

## What USDT is

{% include excalidraw.html
   file="usdt-uprobe"
   alt="USDT probe mechanism: libjvm.so ships hotspot USDT probes described in .note.stapsdt; tools resolve a probe's offset and attach a plain uprobe there, here timing gc__begin to gc__end."
   caption="Figure 20.1 — a USDT probe is a uprobe at a resolved offset" %}

A **USDT** probe (User Statically-Defined Tracepoint) is a marker the
*authors* of a binary placed at a specific point in their code — "a GC
is starting here." At compile time it becomes a no-op instruction plus
an entry in the ELF `.note.stapsdt` section describing the probe's
**name**, its **location** (an instruction offset), and where its
**arguments** live. At runtime, a tracer can attach a uprobe at that
offset; the marker is the contract.

Why this matters for runtimes: the JVM's GC is C++ — mangled symbols,
multiple collector implementations (G1, Parallel, ZGC), internals that
shift between JDK versions and that the JIT complicates. Uprobing those
directly is fragile and collector-specific. The USDT probes
(`hotspot:gc__begin`, `hotspot:gc__end`, and many more) are a **stable,
documented** surface that survives across versions and collectors.
That's the right place to attach.

## A USDT probe is just a uprobe at an offset

Aya has no first-class USDT helper yet — which turns out to be a useful
thing to understand, because it forces the mechanism into the open. A
USDT probe *is* a uprobe at the probe's instruction offset. So we:

1. **Resolve the offset** from the ELF notes. `readelf -n libjvm.so`
   lists each stapsdt probe with its provider, name, and `Location`. The
   demo parses out `gc__begin` and `gc__end`.
2. **Attach a uprobe at that offset** with no symbol name:

```rust
b.attach(None, begin_offset, "/path/libjvm.so", None)?;   // gc__begin
e.attach(None, end_offset,   "/path/libjvm.so", None)?;   // gc__end
```

Passing `None` for the function name and the resolved offset attaches at
exactly the probe site. (libbpf and bpftrace do extra USDT bookkeeping —
semaphore refcounts, argument decoding from the note — that we skip
because we only need begin/end *timing*, not the probe arguments.)

## Timing the pause

`gc__begin` and `gc__end` are **separate** probe sites, so this needs no
uretprobe — two plain uprobes and the entry/exit pattern from
Chapter 18, keyed by pid (a stop-the-world GC pauses the whole JVM):

```rust
#[uprobe] pub fn gc_begin(..) { GC_START.insert(&pid, &ktime(), 0); }
#[uprobe] pub fn gc_end(ctx)  { let pause = ktime() - GC_START.get(&pid)?; emit(pause); }
```

User space records each pause into the OTLP histogram `jvm_gc_pause_ms`.

## What's observable inside the JVM

GC pauses are the headline, but the same mechanism — a uprobe at a USDT
offset — reaches the JVM's other subsystems, because HotSpot instruments
all of them:

{% include excalidraw.html
   file="jvm-observable"
   alt="The JVM (libjvm.so) exposes HotSpot USDT probes across its subsystems: garbage collector (gc__begin/gc__end), memory pools, JIT compiler (method__compile), threads (thread__start/stop), monitors/locks (monitor__contended), allocation (object__alloc) and class loading (class__loaded). Each is reachable as an eBPF uprobe at the probe's offset." %}

So the same technique that times GC also surfaces JIT compilation
(`method__compile__begin`/`__end`), lock contention
(`monitor__contended__*`), allocation rate (`object__alloc`), thread
lifecycle (`thread__start`/`__stop`), and class loading
(`class__loaded`) — each a probe you resolve and attach exactly as we did
`gc__begin`/`gc__end`. List what a given `libjvm.so` carries with
`readelf -n libjvm.so` or `bpftrace -l 'usdt:/path/libjvm.so:*'`.

One note on collectors: the JDK — and the OpenJDK UBI image we build on —
ships **G1** (the default), **ZGC**, and **Shenandoah**, selected with
`-XX:+UseG1GC` / `-XX:+UseZGC` / `-XX:+UseShenandoahGC`. All three fire
`gc__begin`/`gc__end`, so this tool works against any of them — but the
*shape* differs. G1's pauses are the stop-the-world kind you're timing
here; ZGC and Shenandoah are mostly **concurrent**, so their
stop-the-world phases are sub-millisecond and the signal of interest
shifts toward concurrent-cycle frequency and allocation pressure.

## Many JVMs on one node

A real host rarely runs one JVM. Picture several services, each in its
own container, each with its own `libjvm.so` under its own overlay
filesystem — and possibly each on a different collector. Two things
follow from Chapter 16:

- **One probe per binary path.** A uprobe attaches to a *file*, and each
  container's `libjvm.so` is a distinct path (under the container's
  overlay, reachable on the host as `/proc/<pid>/root/.../libjvm.so`). To
  watch every JVM you resolve the USDT offset and attach the probe once
  per distinct `libjvm.so`.
- **Attribute by cgroup / PID.** Events carry the host PID; map it to its
  container via the cgroup id (Chapter 16) and label the metric
  `jvm_gc_pause_ms{container=…,collector=…}`. Now one dashboard compares
  GC behaviour across services — the G1 service's longer pauses beside
  the ZGC service's flat line — which is exactly the kind of fleet view
  per-JVM agents can't give you cheaply.

## Why GC monitoring matters

GC is where JVM performance quietly goes to die, which is why it's one of
the most-watched signals in production Java:

- **Stop-the-world pauses *are* tail latency.** A G1 pause freezes every
  application thread; a 50 ms pause adds 50 ms to whatever requests were
  in flight. Pause time tracks p99/p999 latency directly, and it's
  invisible to in-process timers — the app wasn't running to measure
  itself.
- **Time-in-GC is a saturation signal.** Rising pause *frequency*, or a
  climbing fraction of wall-clock spent in GC, means the heap is too
  small or allocation too high — the lead indicator before an
  `OutOfMemoryError` or a latency cliff.
- **What to alert on:** p99 pause duration (e.g. "G1 pause > 100 ms"),
  pauses per minute, and percent-time-in-GC — all three derivable from
  `jvm_gc_pause_ms`.

The out-of-process angle is the whole point: eBPF reads the JVM's own
USDT probes with **no JVM flags to change in production, no Java agent
inside the process, and no `-verbose:gc` log to parse** — uniformly
across every JVM on the node, whatever collector each one runs.

## The lab-policy note

Our app targets are normally containerized; this chapter runs the JVM
**directly on the VM** to keep the focus on USDT and GC. To observe the
containerized **Quarkus** target from Chapter 16 instead, combine this
with that chapter's container-path resolution — locate `libjvm.so` under
the container's overlay filesystem and point `javagc` at that path. The
probe mechanics are identical; only the path discovery changes.

## Build, deploy, observe

Needs a JDK on the VM (`sudo dnf install -y java-latest-openjdk-devel`):

```bash
cd examples/20-javagc && ./demo.sh
```

The demo compiles a small allocation-heavy `Alloc.java`, runs it with a
deliberately small heap (`-Xmx64m -XX:+UseG1GC -XX:+ExtendedDTraceProbes`)
so GCs are frequent, resolves the USDT offsets, and times pauses. You'll
see per-pause millisecond timings and `jvm_gc_pause_ms` in Grafana — the
exact signal you'd alert on for a latency-sensitive service ("p99 GC
pause crept over 10 ms").

**In Grafana** (`127.0.0.1:3000` → Explore), filter to the `ebpf-javagc` service and graph `sum by (program) (rate(ebpf_events_total[1m]))` — JVM garbage-collection events as a live rate, the same events your terminal lists, now plotted over time.

## Cross-check — bpftrace speaks USDT natively

```bash
[vm]$ sudo bpftrace -e 'usdt:/path/libjvm.so:hotspot:gc__begin { @b[pid]=nsecs }
  usdt:/path/libjvm.so:hotspot:gc__end /@b[pid]/ { @ms=hist((nsecs-@b[pid])/1000000); delete(@b[pid]) }'
```

`bpftrace` resolves USDT probes by `provider:name` directly — no offset
math. It's the guaranteed-working reference: if its pause histogram
matches `javagc`'s, your offset resolution and timing are correct. If
`javagc` can't resolve offsets, this `bpftrace` line still shows you GC
working while you sort the resolution out.

## What you learned

- **USDT** probes are author-placed, stable markers in a binary's
  `.note.stapsdt` — the right surface for runtime-internal events like
  GC, far better than uprobing mangled C++ internals.
- With no first-class USDT helper, a USDT probe is **a uprobe at the
  resolved offset** (`attach(None, offset, lib, None)`).
- Time stop-the-world pauses with two uprobes (begin/end) and the
  entry/exit pattern — no uretprobe needed.

That completes **User-space & language probing**. Next the tutorial
turns to **Performance & resources** — scheduling latency, IRQs,
profiling, memory.

---

*Verification status: <span class="status status--unverified">unverified</span>
— the most experimental chapter so far. Highest-risk: USDT offset
resolution (the `readelf` Location → uprobe file-offset assumption may
need a vaddr conversion), the JDK shipping the hotspot probes, and the
two-uprobe attach in aya 0.13.x. `bpftrace`'s native USDT is the
fallback reference. The first build and run are the test.*
