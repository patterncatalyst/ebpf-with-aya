---
title: "funclatency"
order: 18
part: User-space & language probing
description: Time a function per call by pairing a uprobe at entry with a uretprobe at return and bpf_ktime_get_ns, then turn the durations into an OTLP latency histogram — and weigh per-call events against an in-kernel histogram.
duration: 25 minutes
---

"Why is this slow?" starts with "how long does this function take?"
`funclatency` answers it: pair a uprobe at a function's entry with a
uretprobe at its return, subtract timestamps, and you have a per-call
duration. Collect enough of them and you have a latency distribution —
the thing percentiles and heatmaps are built from. This chapter builds
it and feeds the durations into the observability stack as a real
histogram.

The code is in `examples/18-funclatency/`.

## Timing by pairing entry and return

You met the entry/exit `HashMap` pattern for correlating data
(Chapters 8, 9, 17). Timing is the same skeleton with a clock:

```rust
#[uprobe]
pub fn fn_enter(_ctx: ProbeContext) -> u32 {
    START.insert(&pid_tgid(), &bpf_ktime_get_ns(), 0);   // stash entry time
    0
}

#[uretprobe]
pub fn fn_exit(_ctx: RetProbeContext) -> u32 {
    let start = START.get(&pid_tgid())?;                 // entry time
    let delta = bpf_ktime_get_ns() - start;              // duration
    // emit LatEvent { delta_ns, ... }
    0
}
```

`bpf_ktime_get_ns()` is a monotonic nanosecond clock — exactly what you
want for intervals (it doesn't jump with wall-clock changes). Keying the
start time by `pid_tgid` means each thread's call is timed
independently, so concurrent calls don't clobber each other. The
duration is `now - start`, computed at return.

## Per-call events vs. an in-kernel histogram

There are two ways to get from per-call durations to a distribution, and
the choice is a real engineering trade-off:

1. **Emit one event per call** (what this example does): the uretprobe
   ships each `delta_ns` to user space via a ring buffer, and user space
   records it into a histogram. Simple, and it lets user space drive a
   real OTLP histogram the stack understands. The cost is one event per
   call — fine at modest rates.
2. **Aggregate a histogram in the kernel** (what the classic
   `funclatency` does): keep an array of log2 bucket counters as a map;
   the uretprobe just increments `buckets[log2(delta)]`; user space
   reads the buckets periodically. Almost no per-call overhead — right
   for hot functions called millions of times.

We take approach 1 because it plugs straight into OTLP and is easier to
read; the chapter's example notes where approach 2 swaps in. Knowing
*why* you'd switch — call rate — is the actual lesson.

## Into an OTLP histogram

User space records each duration into an OpenTelemetry **histogram**
instrument:

```rust
let hist = meter.f64_histogram("function_latency_ms").build();
// per event:
hist.record(delta_ns as f64 / 1_000_000.0, &[KeyValue::new("symbol", sym)]);
```

That gives the stack a proper histogram series — Grafana can render a
heatmap or compute p50/p95/p99 without you precomputing them. Alongside,
the tool prints a console ASCII histogram in log2-µs buckets, the way
`funclatency` always has, so you get an immediate read in the terminal
too.

## Build, deploy, observe

The example bundles a `target-app` whose `slow_op()` sleeps a variable
amount, so the distribution has shape:

```bash
cd examples/18-funclatency && ./demo.sh
```

It ships the target to the VM, starts it, and times `slow_op`. Every
couple of seconds you get:

```text
slow_op: 240 calls
      usec    count  distribution
 256 -> 511       40  |********
 512 -> 1023      88  |****************
1024 -> 2047     112  |********************************
```

and `function_latency_ms{symbol="slow_op"}` in Grafana — point a heatmap
panel at it, or a percentile query, and you have the same view you'd
build for a real service. Aim `funclatency` at any symbol in any binary
(`SYM=… ./demo.sh`, or pass a path + symbol) and you can time arbitrary
user-space functions — your code, a library, anything with a symbol.

## Cross-check

```bash
[vm]$ sudo funclatency-bpfcc /home/fedora/target-app:slow_op
```

The BCC tool prints its own log2 histogram for the same function; the
bucket shape should match yours.

## What you learned

- Time a function by stashing `bpf_ktime_get_ns()` at a uprobe entry and
  subtracting at the uretprobe return, keyed per thread.
- Turn durations into an **OTLP histogram** for heatmaps and
  percentiles, with a console histogram for immediate feedback.
- The per-call-event vs. in-kernel-histogram choice is about **call
  rate** — know when to switch.

That closes the *User-space & language probing* part's core
techniques. The remaining chapters in this part (goroutine states,
`javagc`) apply them to specific runtimes; from there the tutorial moves
into **Performance & resources**.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm `bpf_ktime_get_ns`, uprobe+uretprobe on one symbol, the
entry/exit `HashMap`, attachability under release+LTO, and
`f64_histogram` in opentelemetry 0.27. The first build and run are the
test.*
