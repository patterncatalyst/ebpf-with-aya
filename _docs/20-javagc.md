---
title: "javagc"
order: 20
part: User-space & language probing
description: Time JVM garbage-collection pauses — learn what USDT is and why it's the ideal surface, discover that a stock OpenJDK ships no gc USDT markers, and fall back to uprobing the G1 collector's real stop-the-world function resolved from libjvm.so's symbol table.
duration: 30 minutes
---

This chapter times JVM **garbage-collection pauses** — and in doing so
introduces **USDT**, the mechanism language runtimes use to expose
stable internal events. The JVM, CPython, Node, and others *can* ship
USDT probes precisely so tools like ours observe them without reverse-
engineering their internals. But there's a catch we'll hit head-on: a
**stock Fedora OpenJDK isn't built with `--enable-dtrace`, so its
`gc__begin`/`gc__end` markers don't exist**. So this chapter teaches both
halves — what USDT is and why it's the right surface *when it's there*,
and the portable fallback when it isn't: uprobe the collector's real
stop-the-world function, resolved from `libjvm.so`'s symbol table (the
same `.symtab` technique the postgres chapter used).

The code is in `examples/20-javagc/`. `./demo.sh` there builds, deploys,
and runs it; its `README.md` covers what it does and how to drive it.

## What USDT is

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

## The reveal: a stock OpenJDK has no gc USDT markers

A USDT probe *is* a uprobe at the probe's instruction offset — so in
principle you `readelf -n libjvm.so`, read the `gc__begin`/`gc__end`
`Location`s from the `.note.stapsdt` section, and attach a uprobe at
each. Aya has no first-class USDT helper, which is fine: the mechanism is
just uprobe-at-offset.

Except on Fedora it comes up empty:

```console
[vm]$ readelf -n $(find /usr/lib/jvm -name libjvm.so) | grep -c gc__
0
[vm]$ sudo bpftrace -l "usdt:$(find /usr/lib/jvm -name libjvm.so):hotspot:*"
(nothing)
```

The HotSpot USDT probes only exist in an OpenJDK compiled with
`--enable-dtrace` (which needs `systemtap-sdt-devel` at build time).
Fedora's — and most distributions' — OpenJDK is **not** built that way.
There are no markers to attach to. This isn't a bug in our tool; it's the
reality of the JDK you'll actually find on a box.

## The portable fallback: uprobe the collector itself

{% include excalidraw.html
   file="usdt-uprobe"
   alt="Fedora's stock OpenJDK ships no hotspot USDT gc markers, so instead of a USDT probe we resolve G1CollectedHeap::do_collection_pause_at_safepoint from libjvm.so's unstripped .symtab and attach a uprobe at its entry (record start) plus a uretprobe at its return (pause = now − start)."
   caption="Figure 20.1 — no USDT markers on a stock JDK, so uprobe the G1 collector from libjvm's symbol table" %}

If the *marker* isn't there, uprobe the **function the marker would have
sat in**. libjvm.so ships an unstripped `.symtab` (only the `.debug` is
split out via `.gnu_debuglink`), so the collector's C++ symbols are
resolvable — exactly the situation the postgres chapter (47) handled. The
G1 collector's stop-the-world work runs inside:

```
G1CollectedHeap::do_collection_pause_at_safepoint(unsigned long)
```

We uprobe its **entry** to timestamp, and put a **uretprobe** on its
**return** to compute the pause. Unlike the two-marker USDT approach, this
is one function timed entry-to-exit — and it catches *every* automatic G1
pause (young/mixed/full), not just explicit `System.gc()`.

Aya resolves the symbol to a file offset for us — it reads both `.dynsym`
and `.symtab`, so a local C++ symbol is found and the vaddr→offset math is
handled internally. We just hand it the (mangled) name:

```rust
let b: &mut UProbe = ebpf.program_mut("gc_begin").unwrap().try_into()?;  // #[uprobe]
b.load()?;
b.attach(symbol, &libjvm, UProbeScope::AllProcesses)?;                   // entry
let e: &mut UProbe = ebpf.program_mut("gc_end").unwrap().try_into()?;    // #[uretprobe]
e.load()?;
e.attach(symbol, &libjvm, UProbeScope::AllProcesses)?;                   // return
```

The name is JDK-version-specific and mangled
(`_ZN15G1CollectedHeap32do_collection_pause_at_safepointEm` on JDK 26), so
the demo resolves it dynamically with `nm` rather than hard-coding it:

```bash
SYM=$(nm "$LIBJVM" | awk '/G1CollectedHeap.*do_collection_pause_at_safepoint/{print $3; exit}')
```

## Timing the pause

Entry and return are the same function on the same thread, so we key the
start map by **tid** — the JVM's "VM Thread" runs the pause, and its
uprobe entry and uretprobe return share a kernel tid:

```rust
#[uprobe]    pub fn gc_begin(..)  { GC_START.insert(&tid, &ktime(), 0); }
#[uretprobe] pub fn gc_end(ctx)   { let pause = ktime() - GC_START.get(&tid)?; emit(pause); }
```

User space records each pause into the OTLP histogram `jvm_gc_pause_ms`.

## What's observable inside the JVM

GC pauses are the headline, but the same mechanism — a uprobe at a USDT
offset — reaches the JVM's other subsystems, because HotSpot instruments
all of them:

{% include excalidraw.html
   file="jvm-observable"
   alt="The JVM (libjvm.so) exposes HotSpot USDT probes across its subsystems: garbage collector (gc__begin/gc__end), memory pools, JIT compiler (method__compile), threads (thread__start/stop), monitors/locks (monitor__contended), allocation (object__alloc) and class loading (class__loaded). Each is reachable as an eBPF uprobe at the probe's offset." %}

On a **dtrace-enabled** JDK the same technique that times GC also
surfaces JIT compilation (`method__compile__begin`/`__end`), lock
contention (`monitor__contended__*`), allocation rate (`object__alloc`),
thread lifecycle (`thread__start`/`__stop`), and class loading
(`class__loaded`) — each a probe you resolve and attach exactly as USDT
intends. List what a given `libjvm.so` carries with
`readelf -n libjvm.so` or `bpftrace -l 'usdt:/path/libjvm.so:*'`. On a
stock (non-dtrace) JDK like Fedora's, that list is empty — and the same
`.symtab`-uprobe fallback applies: find the C++ function behind the event
you want (`nm libjvm.so | grep …`) and uprobe it, accepting the
JDK-version coupling that USDT was designed to avoid.

One note on collectors: the JDK ships **G1** (the default), **ZGC**, and
**Shenandoah**, selected with `-XX:+UseG1GC` / `-XX:+UseZGC` /
`-XX:+UseShenandoahGC`. We uprobe a **G1-specific** symbol
(`G1CollectedHeap::do_collection_pause_at_safepoint`), so this exact tool
is G1-only — for ZGC/Shenandoah you'd resolve *their* pause functions
instead. And the *shape* differs anyway: G1's pauses are the
stop-the-world kind you're timing here; ZGC and Shenandoah are mostly
**concurrent**, so their stop-the-world phases are sub-millisecond and the
signal of interest shifts toward concurrent-cycle frequency and
allocation pressure.

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

The out-of-process angle is the whole point: eBPF times the JVM's own GC
from outside with **no JVM flags to change in production, no Java agent
inside the process, and no `-verbose:gc` log to parse** — uniformly
across every JVM on the node. On a dtrace-enabled JDK you'd read the
`gc__*` USDT markers; on a stock JDK you uprobe the collector function
directly, as we do here. Either way the process itself is untouched.

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
deliberately small heap (`-Xmx64m -XX:+UseG1GC`) so GCs are frequent,
resolves the G1 pause symbol from `libjvm.so`'s `.symtab`, and times
pauses with the uprobe+uretprobe pair. You'll see per-pause millisecond
timings (`VM Thread  0.97 ms`) and `jvm_gc_pause_ms` in Grafana — the
exact signal you'd alert on for a latency-sensitive service ("p99 GC
pause crept over 10 ms").

**In Grafana** (`127.0.0.1:3000` → Explore), filter to the `ebpf-javagc` service and graph `sum by (program) (rate(ebpf_events_total[1m]))` — JVM garbage-collection events as a live rate, the same events your terminal lists, now plotted over time.

## Cross-check — the same probe under bpftrace

```bash
[vm]$ LJ=$(find /usr/lib/jvm -name libjvm.so | head -1)
[vm]$ SYM=$(nm "$LJ" | awk '/do_collection_pause_at_safepoint/{print $3; exit}')
[vm]$ sudo bpftrace -e "uprobe:$LJ:$SYM { @b[tid]=nsecs }
  uretprobe:$LJ:$SYM /@b[tid]/ { @ms=hist((nsecs-@b[tid])/1000000); delete(@b[tid]) }"
```

`bpftrace` resolves the same symbol from `.symtab` and attaches the same
uprobe+uretprobe pair — the guaranteed-working reference. If its pause
histogram matches `javagc`'s, your symbol resolution and timing are
correct. (On a dtrace-enabled JDK you could instead write
`usdt:$LJ:hotspot:gc__begin` / `gc__end` and let bpftrace resolve the
markers by `provider:name` — but Fedora's JDK has none, which is the
whole reason we uprobe the function.)

## What you learned

- **USDT** probes are author-placed, stable markers in a binary's
  `.note.stapsdt` — the *ideal* surface for runtime-internal events like
  GC, because they survive across JDK versions and collectors.
- But USDT only exists if the binary was built for it — a stock Fedora
  OpenJDK is **not** `--enable-dtrace`, so its `gc__*` markers are absent.
  Always check (`readelf -n | grep gc__`) before assuming.
- The portable fallback is to **uprobe the C++ function itself**, resolved
  from `libjvm.so`'s `.symtab` (aya reads it and does the vaddr→offset
  math), timing entry→return with a **uprobe + uretprobe** — at the cost
  of JDK-version coupling that USDT was designed to avoid.

That completes **User-space & language probing**. Next the tutorial
turns to **Performance & resources** — scheduling latency, IRQs,
profiling, memory.

---

*Verification status: <span class="status status--verified">verified</span>
— Fedora 44, kernel 7.1.3, OpenJDK 26 (Red Hat build). Confirmed the stock
JDK ships **no** hotspot gc USDT markers, then resolved
`_ZN15G1CollectedHeap32do_collection_pause_at_safepointEm` from `.symtab`,
attached uprobe+uretprobe via aya 0.14, and streamed real GC pauses from
the VM Thread (~0.4–6 ms) exported as `jvm_gc_pause_ms`.*
