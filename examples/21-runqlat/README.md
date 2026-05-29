# Example 21 — runqlat (scheduler run-queue latency, in-kernel histogram)

Measure how long tasks wait **runnable but not running** — i.e. queued
for a CPU. High run-queue latency is the signature of CPU saturation.

## What this shows (new techniques)

- **In-kernel histogram aggregation.** Context switches are a hot path,
  so instead of emitting one event per switch (Ch 18's note), the kernel
  increments a **log2-µs histogram** in an `Array` map; user space just
  reads the buckets. Near-zero per-event overhead — this is *the* chapter
  where that technique earns its keep.
- Stitching three **sched tracepoints** into one measurement:
  `sched_wakeup` / `sched_wakeup_new` (becomes runnable → stamp) and
  `sched_switch` (comes on-CPU → record; preempted → re-stamp).
- Exporting **approximate percentiles** (p50/p90/p99) to Grafana via an
  OTLP **observable gauge** (registered once, read at export time).

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # build + deploy + run (histogram every 2s; Ctrl-C to stop)
```

Create scheduling pressure on the VM (more busy tasks than CPUs):

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'for i in $(seq 1 $(( $(nproc) * 4 ))); do (yes >/dev/null &); done; sleep 30; pkill yes'
```

Console shows the classic ASCII histogram; Grafana gets
`runqueue_latency_us{quantile="p50|p90|p99"}` — point a graph at p99 and
watch it spike under load.

## Cross-check (on the VM)

```bash
[vm]$ sudo runqlat-bpfcc 2 5
```

The BCC tool prints the same distribution; bucket shapes should match.

## ⚠ Verification status

**Unverified.** Risks: the `sched_switch`/`sched_wakeup` field offsets
(verify vs. the format files — `prev_pid`@24, `prev_state`@32,
`next_pid`@56, wakeup `pid`@24); `Array::get_ptr_mut` in-kernel
increment; the OTLP **observable gauge** callback API in opentelemetry
0.27; and `TASK_RUNNING == 0` for the preemption check. Record results
in `_plans/reconciliation-plan.md`.
