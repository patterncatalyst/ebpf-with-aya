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
in this part — read the verification notes.

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
profiling, memory. See the
[roadmap]({{ "/plans/iteration-plan/" | relative_url }}).

---

*Verification status: <span class="status status--unverified">unverified</span>
— the most experimental chapter so far. Highest-risk: USDT offset
resolution (the `readelf` Location → uprobe file-offset assumption may
need a vaddr conversion), the JDK shipping the hotspot probes, and the
two-uprobe attach in aya 0.13.x. `bpftrace`'s native USDT is the
fallback reference. The first build and run are the test.*
