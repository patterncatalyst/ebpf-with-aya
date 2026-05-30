---
title: "Energy monitoring"
order: 26
part: Performance & resources
description: "Attribute power consumption to individual processes — the modern sustainability use of eBPF — by crediting each task its on-CPU time and multiplying its share by system power from RAPL, and why virtualization makes this hard."
duration: 30 minutes
---

This chapter closes *Performance & resources* with the newest reason
people reach for eBPF: **energy**. Data-center power is expensive and
carbon-relevant, and "which process is burning the watts?" has become a
real operational question. The CNCF project **Kepler** answers it with
eBPF, and this chapter builds a small version of its core idea —
attributing system power to processes — while being clear about where
the hardware fights you.

The code is in `examples/26-energy/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="energy-attribution"
   alt="Energy attribution: system power (from RAPL, or a model on VMs) is split by each task's CPU-time share to estimate per-process power, exported as estimated_power_watts per comm."
   caption="Figure 26.1 — attributing system power to processes" %}

## The problem: power isn't per-process

A wattmeter (or the CPU's RAPL counters) tells you the *whole package's*
power. The kernel doesn't track "joules per process." So attribution is
a **modeling** problem: split the measured system power among processes
by some proxy for how much work each did. The standard proxies, in
increasing fidelity, are CPU **time**, CPU **cycles**, and a blend of
hardware counters (cycles, instructions, cache misses, DRAM accesses).

Kepler uses hardware counters where it can and falls back to a
utilization model where it can't. We build the utilization model — CPU
**time share** — because it works everywhere, including VMs, and it's
the right place to start.

## Crediting CPU time in eBPF

We've measured on-CPU time before (it underlies `runqlat`). Here we
attribute it per task. On every `sched_switch`, the task **leaving** the
CPU is credited the slice it just ran, and the task **arriving** starts
its clock:

```rust
#[tracepoint] pub fn sched_switch(ctx) {
    let cpu = bpf_get_smp_processor_id();
    let now = bpf_ktime_get_ns();
    if let Some(&start) = ONCPU.get(&cpu) {
        let delta = now - start;
        // credit prev_pid (read prev_comm@8, prev_pid@24) with `delta`
        USAGE[prev_pid].cpu_ns += delta;
    }
    ONCPU.insert(&cpu, &now);   // next task's clock starts now
}
```

`USAGE` accumulates nanoseconds-on-CPU per pid; that's the entire
in-kernel job. The energy *math* lives in user space, where the system
power lives.

## System power: RAPL, when you can get it

Modern Intel and AMD CPUs expose **RAPL** (Running Average Power Limit)
energy counters through `/sys/class/powercap/intel-rapl:*/energy_uj` — a
monotonically increasing microjoule counter. Sample it twice and divide
by the interval to get watts:

```rust
let system_w = (rapl_uj_now.wrapping_sub(rapl_uj_prev) as f64) / 1e6 / dt_secs;
```

Then each process's estimated power is just its share:

```text
power(proc) = system_w × cpu_ns(proc) / cpu_ns(all)
```

User space aggregates by `comm`, prints a table, and exports
`estimated_power_watts{comm}` plus `system_power_watts` to Grafana — a
live "what's costing watts" panel.

## The hard part: virtualization breaks the hardware path

Here's the reality the marketing slides skip: **RAPL is almost never
exposed inside a VM**, and hardware performance counters often aren't
either unless the hypervisor enables a guest **vPMU**. Our lab target is
a KVM guest, so:

- `/sys/class/powercap` is typically **absent** on the VM. The example
  detects this and falls back to a flat `ENERGY_TDP_WATTS` model (set it
  to your CPU's TDP for a better guess). The *attribution* — who used
  what share — is still correct; only the absolute watts are modeled.
- For real hardware energy numbers, run the tool on **bare-metal**
  Fedora where RAPL exists. The eBPF side is identical; only the power
  source changes.
- This is exactly the regime Kepler is built for: in clouds and VMs
  where RAPL is unavailable, it *models* power from utilization and
  counters rather than reading it. Our fallback is a simplified version
  of that same accommodation.

So treat this chapter's watts as an **estimate** — which is all
per-process energy ever is. The technique (credit CPU time in-kernel,
multiply by a system-power source, attribute by share) is the real,
production-shaped lesson; the precision depends on what your hardware and
hypervisor will tell you.

## The accuracy upgrade: hardware counters

For better fidelity on bare metal, swap CPU *time* for CPU *cycles* (and
beyond). That means a `PERF_EVENT_ARRAY` map populated with hardware
"cycles" perf events and `bpf_perf_event_read_value` to read the counter
in-kernel, attributing cycle deltas to tasks on `sched_switch` — then a
power model mapping cycles/instructions to joules. It's more accurate and
more hardware-dependent (needs PMU access); the CPU-time model here is
the portable floor. The README points at how to extend toward it.

## Build, deploy, observe

```bash
cd examples/26-energy && ./demo.sh
```

Spin up some CPU burners on the VM (`sha256sum /dev/zero`, a busy loop)
and watch them rise to the top of the power table, with
`estimated_power_watts{comm}` tracking them in Grafana.

## Cross-check

```bash
[vm]$ ls /sys/class/powercap/intel-rapl:0/energy_uj 2>/dev/null || echo "no RAPL (expected on a VM)"
[host]$ sudo turbostat --interval 2     # bare-metal package power, to sanity-check the model
```

## What you learned

- Per-process energy is an **attribution model**, not a measurement:
  split system power by a work proxy (CPU time → cycles → counters).
- Credit on-CPU time per task in-kernel via `sched_switch`; do the energy
  math in user space.
- Read system power from **RAPL** when available; **model** it when not —
  the VM reality, and Kepler's reality in the cloud.

That closes **Performance & resources**. The signals built here —
latency, profiles, power — are exactly what a **`sched_ext`** scheduler
(Part 6) can consume to make power- and QoS-aware decisions; first,
Part 4 takes on **networking**.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Risks: the `sched_switch` offsets, RAPL path/availability (absent on most
VMs — fallback model used), the observable-gauge API in opentelemetry
0.27, and the inherent approximation of the attribution model. The first
build and run are the test.*
