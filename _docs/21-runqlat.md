---
title: "runqlat"
order: 21
part: Performance & resources
description: "Measure scheduler run-queue latency — how long tasks wait runnable-but-not-running — by stitching together the sched tracepoints, and meet the technique that hot paths demand: aggregating a histogram in the kernel instead of emitting per-event."
duration: 30 minutes
---

The *Performance & resources* part opens with a question every busy
system eventually raises: "my CPU isn't pegged, so why is everything
slow?" Often the answer is **run-queue latency** — tasks are *ready* to
run but stuck waiting for a CPU to free up. `runqlat` measures exactly
that wait, and building it teaches the technique that separates toy
tracers from production ones: **aggregating in the kernel**.

The code is in `examples/21-runqlat/`.

{% include excalidraw.html
   file="runqlat-timeline"
   alt="Run-queue latency: sched_wakeup stamps when a task becomes runnable; it waits on the run queue; sched_switch records the wait when it comes on-CPU, bucketed into an in-kernel log2 histogram."
   caption="Figure 21.1 — measuring run-queue latency across the sched tracepoints" %}

## What run-queue latency is

A task's life cycle on the scheduler: it sleeps, something **wakes** it
(it becomes *runnable*), it sits on a CPU's run queue, and eventually the
scheduler **switches** it onto a CPU (it becomes *running*). The gap
between "became runnable" and "started running" is run-queue latency —
pure waiting for a CPU. When more tasks want to run than you have CPUs,
this number climbs even while utilization looks fine. It's the
single best signal for CPU saturation.

## Stitching three tracepoints

No single tracepoint gives you the wait, so we combine three from the
`sched` subsystem:

- **`sched_wakeup`** and **`sched_wakeup_new`** — a task became runnable.
  Stamp the current time against its pid (`START[pid] = now`).
- **`sched_switch`** — a context switch. Two things to do:
  - The task coming **on** the CPU (`next_pid`): its wait just ended, so
    `delta = now - START[next_pid]`, bucket it, clear the stamp.
  - The task going **off** the CPU but still runnable (`prev_state ==
    TASK_RUNNING`, i.e. preempted rather than blocked): it's going
    *back* into the queue, so re-stamp it.

```rust
#[tracepoint] pub fn sched_wakeup(ctx)      { stamp(ctx.read_at::<i32>(WAKE_PID)?); }
#[tracepoint] pub fn sched_switch(ctx) {
    if prev_state == TASK_RUNNING { stamp(prev_pid); }  // preempted -> re-queued
    record(next_pid);                                    // on-CPU -> close the wait
}
```

The field offsets (`prev_pid` @ 24, `prev_state` @ 32, `next_pid` @ 56,
wakeup `pid` @ 24) come from the tracepoint format files, the same way
as Chapters 9–12 — and they're the first thing to verify.

## The real lesson: aggregate in the kernel

Back in Chapter 18 we emitted one ring-buffer event per function call
and noted: *fine at modest rates, but for hot paths aggregate in the
kernel instead.* `sched_switch` is the definitive hot path — it fires on
**every context switch on every CPU**, thousands of times a second on a
busy box. Shipping an event each time would drown user space and perturb
the very scheduling you're measuring.

So `runqlat` keeps a **log2 histogram in an `Array` map**: bucket *i*
counts waits in `[2^i, 2^(i+1))` microseconds. The `sched_switch`
handler computes the bucket and bumps a counter in place:

```rust
let us = (now - start) / 1000;
let bucket = (63 - us.max(1).leading_zeros()).min(NBUCKETS - 1);  // floor(log2)
if let Some(slot) = HIST.get_ptr_mut(bucket) { unsafe { *slot += 1; } }
```

That's the entire per-event cost: a subtraction, a `leading_zeros`, and
one increment — no allocation, no copy to user space. User space reads
the 27 bucket counters whenever it likes (every two seconds here). This
is how the real `runqlat`, `biolatency`, and friends are built.

## To the console, and to Grafana

User space does two things with the buckets. It prints the classic ASCII
histogram:

```text
run-queue latency (usec)   total=48213
         4 -> 7          1842 |****
         8 -> 15        12903 |********************************
        16 -> 31         9011 |**********************
       ...
```

And it exports **approximate percentiles** to the stack. Percentiles
from a histogram are inherently approximate (you know the bucket, not
the exact value), so we report each bucket's upper edge for p50/p90/p99
and publish them as an OTLP **observable gauge**:

```rust
let _gauge = meter.f64_observable_gauge("runqueue_latency_us")
    .with_callback(move |obs| {
        let b = *snap.lock().unwrap();
        for (q, name) in [(0.50,"p50"),(0.90,"p90"),(0.99,"p99")] {
            obs.observe(percentile_us(&b, q), &[KeyValue::new("quantile", name)]);
        }
    }).build();
```

The observable gauge is the right OTLP instrument here: it's
**registered once**, and the SDK invokes the callback at each export to
read the latest snapshot — no per-event recording. Point a Grafana graph
at `runqueue_latency_us{quantile="p99"}` and you have a live p99
run-queue-latency line that jumps the moment the box runs out of CPU.

## Build, deploy, observe

```bash
cd examples/21-runqlat && ./demo.sh
```

Then make some scheduling pressure on the VM — more busy tasks than CPUs:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'for i in $(seq 1 $(( $(nproc) * 4 ))); do (yes >/dev/null &); done; sleep 30; pkill yes'
```

Watch the histogram shift right and p99 climb while the `yes` storm
runs, then settle when it stops.

## Cross-check

```bash
[vm]$ sudo runqlat-bpfcc 2 5
```

The BCC tool prints the same distribution; the bucket shapes should
match yours.

## What you learned

- **Run-queue latency** = time runnable-but-not-running; the clearest
  CPU-saturation signal.
- Measure it by stitching `sched_wakeup`/`_new` (stamp) and
  `sched_switch` (record on-CPU, re-stamp preempted).
- For hot paths, **aggregate a histogram in the kernel** (`Array` +
  `get_ptr_mut`) instead of emitting per-event — the technique behind
  every production `*lat` tool.
- Export histogram percentiles with an OTLP **observable gauge**.

Next: **`hardirqs`**, timing the kernel's hardware-interrupt handlers.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Highest-risk: the sched tracepoint field offsets, `Array::get_ptr_mut`
in-kernel increment, `TASK_RUNNING == 0`, and the observable-gauge
callback API in opentelemetry 0.27. The first build and run are the
test.*
