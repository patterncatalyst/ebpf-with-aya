---
title: "hardirqs"
order: 22
part: Performance & resources
description: "Measure the time the kernel spends in hardware-interrupt handlers, per IRQ vector, by timing the irq tracepoints keyed by CPU and accumulating per-vector totals in a kernel map — and understand why hardirq time is hidden latency."
duration: 25 minutes
---

Hardware interrupts are the kernel dropping everything to service a
device — a packet arrived, a disk finished, a timer fired. Handlers are
supposed to be fast, but when a device is busy, the *aggregate* time in
hardirq context can become significant, and it's stolen from whatever
was running. `hardirqs` measures it per IRQ vector, so you can see which
device is costing you. It also reinforces the in-kernel-aggregation
technique from `runqlat`, with a per-CPU twist.

The code is in `examples/22-hardirqs/`.

## Why hardirq time is hidden

A CPU servicing a hardware interrupt isn't running your task, isn't
counted as your task's CPU time, and the handler can't be preempted.
A flood of interrupts — a busy NIC, a thrashing disk — burns CPU in
short bursts that don't show up as any process's usage but absolutely
show up as latency. `top` looks calm; your tail latencies don't. Seeing
hardirq time **per vector** points straight at the offending device.

## Timing the handlers, keyed by CPU

Two tracepoints bracket every handler:

- **`irq:irq_handler_entry`** — a handler is about to run. It carries the
  IRQ number (`irq` field).
- **`irq:irq_handler_exit`** — it finished (also carries `irq`, plus the
  handler's return value).

The timing is the familiar entry/exit pattern, but the key is different.
Earlier tools keyed by pid or pid_tgid; an IRQ handler isn't associated
with a task — it runs in **interrupt context on a specific CPU**. So we
key the start timestamp by **CPU id**:

```rust
#[tracepoint] pub fn irq_handler_entry(_ctx) {
    START.insert(&bpf_get_smp_processor_id(), &bpf_ktime_get_ns(), 0);
}
#[tracepoint] pub fn irq_handler_exit(ctx) {
    let delta = bpf_ktime_get_ns() - START.get(&cpu)?;
    let irq = ctx.read_at::<i32>(IRQ_OFF)?;     // which vector
    // add delta + 1 to HIST[irq]
}
```

`bpf_get_smp_processor_id()` returns the CPU the handler is running on;
since a handler runs to completion on that CPU, the entry and exit see
the same CPU, so the stamp matches up.

> **Nested IRQs — a documented simplification.** A higher-priority
> interrupt can interrupt a handler already running on the same CPU. Our
> single-slot-per-CPU `START` map doesn't disentangle that nesting (the
> inner IRQ overwrites the outer's stamp). For per-vector *totals* this
> is close enough and matches how the simple `hardirqs` tool behaves;
> precise nesting handling would need a small per-CPU stack. Knowing the
> limitation is the point.

## Accumulating per-vector totals in the kernel

Like `runqlat`, IRQs are a hot path, so we aggregate in the kernel
rather than emit events. The map is a `HashMap` keyed by IRQ number,
holding a `count` and `total_ns`:

```rust
let updated = match HIST.get(&irq) {
    Some(s) => IrqStat { count: s.count + 1, total_ns: s.total_ns + delta },
    None     => IrqStat { count: 1,           total_ns: delta },
};
HIST.insert(&irq, &updated, 0);
```

Read-modify-write on a shared map has a small race window under
concurrency (two CPUs updating different IRQs is fine; the same IRQ from
two CPUs at once could lose an update). For accumulating totals that's
an acceptable approximation — flagged in the verification notes, and the
kind of trade-off you make consciously.

## To the console, and to Grafana

User space iterates the BPF `HashMap` each interval, sorts by total time,
and prints a table:

```text
IRQ          COUNT      TOTAL(us)      AVG(ns)
27           48213          61240         1270
19            8801          12903         1466
```

and exports each vector's total as an OTLP observable gauge
`hardirq_total_ns{irq="27"}`. Cross-reference the IRQ numbers with
`/proc/interrupts` on the VM to put device names to the vectors — IRQ 27
might be your virtio NIC, IRQ 19 the disk controller.

## Build, deploy, observe

```bash
cd examples/22-hardirqs && ./demo.sh
```

Then make interrupts happen on the VM — flood-ping the gateway and push
some direct disk I/O:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'ping -f -c 5000 GATEWAY >/dev/null 2>&1; dd if=/dev/zero of=/tmp/f bs=1M count=512 oflag=direct 2>/dev/null; sync; rm -f /tmp/f'
```

The network and block IRQ vectors should jump to the top of the table.

## Cross-check

```bash
[vm]$ sudo hardirqs-bpfcc 2 5
[vm]$ cat /proc/interrupts
```

## What you learned

- **Hardirq time** is CPU stolen in interrupt context — invisible to
  per-process accounting, visible as latency.
- Time IRQ handlers with `irq_handler_entry`/`exit`, keyed by **CPU**
  (not task), because they run in interrupt context.
- Accumulate **per-vector totals in a kernel `HashMap`**; read and
  export them periodically.
- Be explicit about the **nested-IRQ** and read-modify-write
  simplifications — they're conscious trade-offs, not bugs.

Next: sampling the CPU with a **profiler** (`profile`) to see *where*
time goes, not just how long things wait. See the
[roadmap]({{ "/plans/iteration-plan/" | relative_url }}).

---

*Verification status: <span class="status status--unverified">unverified</span>.
Highest-risk: the `irq` field offset, per-CPU keying under nested IRQs,
`HashMap` read-modify-write under concurrency, and the observable-gauge
API in opentelemetry 0.27. The first build and run are the test.*
