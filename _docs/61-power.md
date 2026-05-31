---
title: "Power and efficiency: attributing energy with eBPF"
order: 61
part: Operating eBPF
description: "The OS bills a process for CPU time, never for joules — so per-workload energy is invisible to ordinary tools. RAPL exposes socket-level energy, and eBPF supplies the missing piece: low-overhead per-workload CPU-time shares to divide that energy by. Learn the powercap interface, the attribution model and its honest limits, where Kepler and the field stand in 2026, and the cpufreq_ext frontier where eBPF starts to control power, not just measure it."
duration: 40 minutes
---

Efficiency has quietly become an operating concern on par with latency:
power is cost, carbon, and the thermal headroom that decides how dense you can
pack a rack. But there's a measurement gap at the centre of it. The OS bills a
process for **CPU time**; it never bills it for **joules**. Ask "which container
is burning the most *power*?" and standard tools shrug — they can tell you CPU
percentages, not watts. eBPF is what closes that gap, and it closes it from both
ends: it attributes hardware energy to individual workloads with low overhead,
and, at the frontier, it's beginning to *control* power rather than just observe
it. This is a fitting near-final chapter for operating eBPF, because efficiency
is where observability turns into action.

The code is in `examples/61-power/`. `./demo.sh` measures per-workload CPU-time
shares in the kernel and, where the hardware exposes it, turns them into watts;
the `README.md` has the details.

{% include excalidraw.html
   file="power"
   alt="Attribute hardware energy to workloads — measure shares with eBPF, divide RAPL. On the left, RAPL via powercap provides energy_uj per domain (package, core, uncore, dram) at socket level, not per-process; and eBPF on sched_switch provides per-workload on-CPU time or cycles via perf counters. These feed an attribution step in the centre: watts times workload_time divided by total_time gives per-workload watts, exported to Grafana as ebpf_estimated_watts per workload. Limits: it ignores DVFS frequency and C-states, so it is comparative, not absolute. The control frontier is cpufreq_ext, which writes frequency policy in eBPF via struct_ops."
   caption="Figure 61.1 — RAPL gives total socket energy; eBPF gives the per-workload weights to divide it by" %}

## Where the joules come from: RAPL

Modern CPUs expose their own energy meters through **RAPL** (Running Average
Power Limit), an Intel feature since Sandy Bridge (AMD has its own). RAPL reports
cumulative energy for several **domains** — `package` (the whole socket),
`core`/PP0 (all cores), `uncore`/PP1 (last-level cache, integrated GPU), and
`dram` — and Linux surfaces it through the **powercap** sysfs framework:

```bash
[host]$ cat /sys/class/powercap/intel-rapl:0/energy_uj      # microjoules, monotonic
[host]$ cat /sys/class/powercap/intel-rapl:0/name           # "package-0"
```

The counter is a monotonic microjoule total; difference two readings over a
known interval and you have average power in watts. Two caveats define
everything that follows:

- It is **socket-level, not per-process.** RAPL knows the package burned 42 W;
  it has no idea which process did. Some server CPUs expose only `package`, not
  `core` — coverage varies by model.
- It is **frequently absent in virtual machines.** RAPL lives in MSRs/powercap
  the hypervisor usually doesn't pass through, so on the KVM guest this book
  uses, `/sys/class/powercap/intel-rapl` may not exist at all. The energy number
  is a bare-metal thing; the lab can still do the *attribution*, just without the
  multiplier.

## The attribution model — and its honest limits

RAPL gives a total; eBPF gives the shares; the estimate is a proportion:

```
energy(workload) ≈ energy(package) × cpu_time(workload) / cpu_time(total)
```

That is, measure how much on-CPU time (or, more precisely, how many cycles) each
workload accumulated, and split the socket's measured energy across them in
proportion. The eBPF half is exactly what this book is good at: a `sched_switch`
tracepoint accumulates per-workload on-CPU time in the kernel, with the
low-overhead, no-polling character that makes it viable in production — the
approach behind **Kepler**, **DEEP-mon**, and the sub-microsecond **Wattmeter**.

It's essential to be candid about what this model ignores, because vendors often
aren't:

- **Frequency scaling (DVFS).** A core at 3.5 GHz draws far more than at 1.2 GHz,
  but CPU *time* doesn't see frequency — two tasks with equal time at different
  frequencies get equal energy here, wrongly.
- **Idle power and C-states.** Energy is spent even when "idle"; a pure
  time-proportional split misattributes some of it.
- **Per-operation variance.** A cache-missing, vector-heavy loop costs more per
  cycle than a stall.

So treat the output as **comparative, not absolute**: excellent for "service A
uses 3× the energy of service B" and for spotting regressions, unreliable as a
billing-grade watt figure. This is precisely the critique levelled at energy
exporters — independent evaluations found cluster-total accuracy wanting — and
it's why the field is moving toward measured sources over pure models (below).

## Where the field is: Kepler and friends

**Kepler** (Kubernetes Efficient Power Level Exporter, a CNCF sandbox project)
is the production face of this: a Prometheus exporter that attributes energy to
containers, pods, VMs, and processes using eBPF tracepoints plus RAPL, dividing
energy by CPU-time. Worth knowing its trajectory, because it illustrates the
accuracy story directly: earlier Kepler leaned on a **trained ML model** to
estimate power where RAPL was unavailable, which drew pointed accuracy criticism;
the project **rewrote for its 0.10 line** (the 0.9.x series is now frozen),
leaning on *measured* sources — RAPL/powercap, HWMon, NVIDIA GPU, and platform
power via Redfish BMC — rather than the model. The lesson for an operator: prefer
a real sensor to an estimate, and know which one your numbers came from.

Around it sit research efforts — **Wattmeter** (context-switch eBPF reading RAPL
MSRs, <1 µs overhead), **DEEP-mon** (in-kernel scheduler-event aggregation) — and
non-eBPF agents like **Scaphandre**. The common thread is the same equation; the
eBPF ones win on overhead by doing the accounting in the kernel instead of
polling `/proc`.

## eBPF as the efficient choice — and the control frontier

Efficiency cuts two ways here. First, **eBPF is itself the low-energy way to
observe**: an in-kernel, event-driven `sched_switch` accumulator costs a
fraction of a userspace agent scraping `/proc` on a timer, so replacing polling
telemetry with eBPF *reduces* the overhead you're trying to measure — and
profiling (Chapter 23) then points you at the workload hotspots worth cutting.

Second, and newer, eBPF is starting to **control** power. **`cpufreq_ext`** is
the first upstream-bound eBPF interface that can set CPU frequency through a
`bpf_struct_ops` (Chapter 55) — a frequency-scaling governor written in eBPF
instead of kernel C, composable with the `sched_ext` schedulers of Part 6. That
turns this chapter's measurement loop into a feedback loop: observe per-workload
energy, then steer frequency or scheduling to reduce it, all in eBPF. It's
early, but it's the direction — eBPF moving from the energy *dashboard* to the
energy *thermostat*.

## Build, deploy, observe

```bash
cd examples/61-power && ./demo.sh
```

The program accumulates per-command on-CPU time on every `sched_switch`. The
loader reads those shares and, **if** `/sys/class/powercap/intel-rapl` exists,
reads package energy and prints estimated watts per command; on the VM (no RAPL)
it prints the CPU-time shares themselves — the attribution weights that *would*
be multiplied by package power on bare metal. **In Grafana**, graph
`ebpf_estimated_watts` (bare metal) or `ebpf_oncpu_seconds_total` (the VM
fallback) to rank workloads by what they cost.

## Cross-check

```bash
[host]$ cat /sys/class/powercap/intel-rapl:0/energy_uj      # the raw socket counter
[host]$ sudo turbostat --interval 1                          # independent package watts
[host]$ perf stat -a -e power/energy-pkg/ sleep 1            # perf's RAPL view
[vm]$   ls /sys/class/powercap/intel-rapl 2>/dev/null || echo "no RAPL in this VM (expected)"
```

On bare metal, your estimated total tracking `turbostat`'s package watts is the
sanity check; in the VM, the *absence* of powercap is itself the lesson — the
eBPF shares are real, the joules need hardware the guest doesn't see.

## What you learned

- The OS bills CPU **time**, not **energy**; **RAPL** (via the **powercap**
  sysfs `energy_uj` counters) measures **socket-level** joules but knows nothing
  about processes — and is usually absent in VMs.
- eBPF supplies the missing per-workload **shares** (on-CPU time/cycles via a
  `sched_switch` accumulator), and `energy × share / total` attributes power to
  workloads — **comparative, not absolute**, because it ignores DVFS and
  C-states; **Kepler** (rewritten at 0.10 toward measured sources after
  ML-estimate accuracy criticism) is the production exporter.
- eBPF is both the **low-overhead way to measure** efficiency and, via
  **`cpufreq_ext`** (struct_ops), an emerging way to **control** it — closing
  the loop from observing energy to steering it.

Next, Chapter 62 closes the book: a look back across everything from a kprobe
counting `unlink` to operating a fleet, and where eBPF and Aya go from here.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: the `sched_switch` field offsets for
`prev_comm`/`prev_pid` on this kernel; that per-command on-CPU time accumulates;
that the loader reads `/sys/class/powercap/intel-rapl:0/energy_uj` where present
and falls back cleanly where not (expect absent in the VM — verify on bare metal
for real watts); and compare any watt estimate against `turbostat`/`perf
power/energy-pkg/`.*
