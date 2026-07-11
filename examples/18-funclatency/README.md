# Example 18 — funclatency (function latency via uprobe + uretprobe)

Measure how long a function takes, per call, and build a latency
distribution — the timing foundation under every "why is this slow"
investigation.

## What this shows

- Pairing **uprobe (entry)** and **uretprobe (return)** to time a
  function: stash `bpf_ktime_get_ns()` at entry keyed by `pid_tgid`,
  subtract at return.
- Feeding each duration into an **OTLP histogram** (`function_latency_ms`)
  so Grafana can draw a heatmap / percentiles, plus a console ASCII
  histogram (log2 µs buckets) like the classic `funclatency`.
- A bundled `target-app` whose `slow_op()` has deliberately variable
  latency, so the distribution is interesting.

## Run it

```bash
./demo.sh build              # build snoop + target-app
./demo.sh                    # ship target-app to VM, start it, time slow_op
SYM=slow_op ./demo.sh        # (default) override the symbol to time
```

Console output every 2s:

```
slow_op: 240 calls
      usec    count  distribution
 256 -> 511       40  |********
 512 -> 1023      88  |****************
1024 -> 2047     112  |********************************
```

and `function_latency_ms{symbol="slow_op"}` in Grafana (a heatmap or
p50/p95/p99 panel).

## In-kernel histogram vs. per-call events

This emits one ring-buffer event per call, which is simple and lets user
space drive a real OTLP histogram. A production `funclatency` aggregates
a **log2 histogram in kernel** (an array map of bucket counters) and
user space just reads the buckets periodically — far less overhead at
high call rates. The chapter covers both; start here, optimize if the
event rate demands it.

## Cross-check (on the VM)

```bash
[vm]$ sudo funclatency-bpfcc -p $(pgrep -f /home/fedora/target-app) /home/fedora/target-app:slow_op
[vm]$ sudo bpftrace -e 'uprobe:/home/fedora/target-app:slow_op { @s[tid]=nsecs } uretprobe:/home/fedora/target-app:slow_op /@s[tid]/ { @ns=hist(nsecs-@s[tid]); delete(@s[tid]) }'
```

## ⚠ Verification status

**Unverified.** Confirm: `bpf_ktime_get_ns`, `#[uprobe]`/`#[uretprobe]`
on the same symbol, and the entry/exit `HashMap` in aya 0.14.x; that
`slow_op` stays attachable under release+LTO (`#[inline(never)]`); and
`f64_histogram` in opentelemetry 0.27. Record results in
`_plans/reconciliation-plan.md`.
