# Example 22 — hardirqs (hardware-IRQ handler time, per vector)

Measure how much time the kernel spends in **hardware-interrupt
handlers**, broken down by IRQ vector. High hardirq time steals cycles
from everything else and shows up as mysterious latency.

## What this shows

- The `irq:irq_handler_entry` / `irq:irq_handler_exit` tracepoints, timed
  with the entry/exit pattern — but **keyed by CPU** (`bpf_get_smp_processor_id`),
  because IRQ handlers run per-CPU, not per-task.
- **In-kernel aggregation** again (like runqlat): a `HashMap<irq, IrqStat>`
  accumulates `count` + `total_ns` per vector in the kernel; user space
  reads the map. No per-IRQ events.
- Iterating a BPF `HashMap` from user space, and exporting per-key values
  as an OTLP observable gauge `hardirq_total_ns{irq=...}`.

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # build + deploy + run (per-IRQ table every 2s; Ctrl-C to stop)
```

Drive interrupts on the VM (network + disk):

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'ping -f -c 5000 _GW_ >/dev/null 2>&1; dd if=/dev/zero of=/tmp/f bs=1M count=512 oflag=direct 2>/dev/null; sync; rm -f /tmp/f'
```

Console table, sorted by total time:

```
IRQ          COUNT      TOTAL(us)      AVG(ns)
27           48213          61240         1270
19            8801          12903         1466
```

and `hardirq_total_ns{irq="27"}` in Grafana.

## Cross-check (on the VM)

```bash
[vm]$ sudo hardirqs-bpfcc 2 5
[vm]$ cat /proc/interrupts          # map IRQ numbers to devices
```

## ⚠ Verification status

**Unverified.** Risks: the `irq` field offset (@8) in the irq tracepoint
format; per-CPU keying vs. **nested IRQs** (a higher-priority IRQ
interrupting a handler on the same CPU isn't disentangled — a documented
simplification); `HashMap` read-modify-write under concurrency (last
write wins, acceptable for accumulation here but verify); and the OTLP
observable-gauge API in opentelemetry 0.27. Record results in
`_plans/reconciliation-plan.md`.
