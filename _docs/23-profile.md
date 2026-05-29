---
title: "profile"
order: 23
part: Performance & resources
description: "Build a sampling CPU profiler with a perf_event program firing at a fixed frequency across all CPUs, capture kernel and user call stacks with bpf_get_stackid and StackTrace maps, and emit flame-graph-ready folded output."
duration: 35 minutes
---

`runqlat` and `hardirqs` told you *how long* things waited. A profiler
tells you *where* the CPU actually spends its time — which functions are
on-CPU when you look. `profile` does it by **sampling**: a timer fires at
a fixed rate on every CPU, and each time it grabs the current call stack.
Aggregate thousands of those samples and the hot paths emerge. This
chapter introduces two new things: the **`perf_event`** program type and
**stack walking**.

The code is in `examples/23-profile/`.

## Sampling, not tracing

Every tool so far attached to a *specific event* — a syscall, a
function, a tracepoint. A profiler is different: it attaches to a
**timer** and asks "what's running *right now*?" at a fixed frequency
(99 Hz per CPU is the convention — 99 rather than 100 to avoid beating
against periodic kernel activity). The cost is therefore **fixed**: 99
samples per second per CPU, whether the box is idle or on fire. That
bounded overhead is what makes sampling profilers safe to run in
production, and it's a fundamentally different cost model from the
per-event tracers.

## The perf_event program type

`perf_event` programs attach to the kernel's performance-monitoring
subsystem. You can hang them off hardware counters (cache misses,
cycles) or software events; for a wall-clock CPU profiler we use the
software event `PERF_COUNT_SW_CPU_CLOCK` with a **frequency** sample
policy. User space sets one up per CPU:

```rust
let prog: &mut PerfEvent = ebpf.program_mut("profile_cpu").unwrap().try_into()?;
prog.load()?;
for cpu in online_cpus()? {
    prog.attach(
        PerfTypeId::Software,
        PERF_COUNT_SW_CPU_CLOCK,                 // config = 0
        PerfEventScope::AllProcessesOneCpu { cpu },
        SamplePolicy::Frequency(99),
    )?;
}
```

`AllProcessesOneCpu { cpu }` means "all processes, this one CPU" — repeat
for every online CPU and you sample the whole machine. The eBPF side is
a `#[perf_event]` function that runs on each tick.

## Walking the stack

On each sample we want the **call stack** — the chain of return
addresses showing who called whom. eBPF captures it with
`bpf_get_stackid`, which walks the stack and stores the frames in a
special **`StackTrace`** map, returning an integer id for that stack:

```rust
let kstack = STACKS.get_stackid(&ctx, 0).unwrap_or(-1) as i32;                 // kernel
let ustack = STACKS.get_stackid(&ctx, BPF_F_USER_STACK as u64).unwrap_or(-1) as i32; // user
```

Kernel and user stacks are captured separately (the flag selects which).
The clever part is that **identical stacks get the same id** — the map
dedups them — so we don't store the same stack a thousand times. Our
count map is keyed by `(pid, comm, kstack_id, ustack_id)`, and the value
is simply how many samples landed on that exact stack:

```rust
let next = COUNTS.get(&key).copied().unwrap_or(0) + 1;
COUNTS.insert(&key, &next, 0);
```

That's the whole hot path: two stack captures and an increment, at a
fixed 99 Hz.

## Symbolizing: addresses → names

The stacks are just **addresses**. To be useful they need to become
function names, and the two halves are resolved differently:

- **Kernel** frames: `/proc/kallsyms` maps kernel addresses to symbol
  names. Aya hands you that as a sorted map via
  `aya::util::kernel_symbols()`; for a frame address you take the
  greatest symbol whose address is ≤ it. (Because `profile` runs *on the
  VM*, that's the right kernel's symbols.)
- **User** frames: this needs the target binary's symbol table and load
  address — a real symbolizer's job (`blazesym`, `addr2line`,
  `perf`'s). This example prints user frames as **hex** and leaves
  wiring in `blazesym` as an exercise; the kernel side shows the
  technique end-to-end.

## Folded output and flame graphs

The output format is **folded stacks** — the lingua franca of flame
graphs:

```text
sha256sum;0x401a30;0x7f...;__x64_sys_read_[k];vfs_read_[k] 412
swapper;native_safe_halt_[k];default_idle_[k] 1880
```

Each line is `comm;frame;frame;… count`, leaf-to-root, kernel frames
suffixed `_[k]`. Pipe it straight into Brendan Gregg's `flamegraph.pl`:

```bash
./demo.sh > out.folded
flamegraph.pl out.folded > cpu.svg
```

and you have the classic flame graph — wide boxes are where the CPU
lives.

## Continuous profiling: Pyroscope

For one-off investigation, folded → flame graph is perfect. For *always
on* profiling, the `otel-lgtm` stack already bundles **Pyroscope**; the
production move is to push these profiles (in pprof format) to it and use
Grafana's flame-graph panel. That ingest path is an extension — the
sampling and stack-capture mechanics here are exactly what feeds it.

## Build, deploy, observe

```bash
cd examples/23-profile && SECS=10 ./demo.sh > out.folded
```

Drive some CPU on the VM while it samples (`sha256sum /dev/zero`), then
build the flame graph from `out.folded`.

## Cross-check

```bash
[vm]$ sudo profile-bpfcc -F 99 10
[vm]$ sudo perf record -F 99 -a -g -- sleep 10 && sudo perf script | stackcollapse-perf.pl
```

Both produce comparable folds; the hot functions should agree.

## What you learned

- A **`perf_event`** program samples on a timer at fixed cost — the
  profiler's cost model, unlike per-event tracing.
- Capture call stacks with `bpf_get_stackid` into a **`StackTrace`**
  map; identical stacks dedup to one id.
- Symbolize kernel frames via `/proc/kallsyms`; user frames need a real
  symbolizer.
- Emit **folded stacks** for flame graphs; push to **Pyroscope** for
  continuous profiling.

Next: **`memleak`** and **`biopattern`** — tracking allocations that
never free, and block-I/O access patterns. See the
[roadmap]({{ "/plans/iteration-plan/" | relative_url }}).

---

*Verification status: <span class="status status--unverified">unverified</span>.
Highest-risk: the `PerfEvent::attach` signature / `SamplePolicy` /
`PerfEventScope` and `online_cpus()` in aya 0.13.x; `get_stackid` (ebpf)
and `StackTraceMap::get().frames()` (user); user-stack capture depending
on frame pointers/unwind info. The first build and run are the test.*
