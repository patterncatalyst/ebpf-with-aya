# Example 26 — energy (per-process power attribution)

Estimate how much **power** each process is responsible for — the modern
"sustainability / efficiency" use of eBPF, as popularized by
[Kepler](https://sustainable-computing.io/).

## What this shows

- Attributing energy by **CPU-time share**: a `sched_switch` tracepoint
  credits each task the time it spends on-CPU (per-task `cpu_ns` in a
  `HashMap`); each task's share of total CPU time × system power = its
  estimated power. This is Kepler's **utilization model**, used in clouds
  where hardware energy counters aren't exposed.
- Reading **system power** from **RAPL** (`/sys/class/powercap/.../energy_uj`)
  when available, with a flat-TDP fallback model when it isn't.
- Aggregating by `comm` and exporting `estimated_power_watts{comm}` +
  `system_power_watts` to Grafana.

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # deploy + run (power-by-process table every 2s)
```

Make some CPU consumers on the VM:

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'timeout 30 sha256sum /dev/zero & timeout 30 md5sum /dev/zero'
```

```
system ~15.00 W   (busy 1980.4 ms/interval)
COMM               SHARE%      WATTS
sha256sum           48.2%       7.23
md5sum              39.1%       5.87
```

## ⚠ Important: VMs, RAPL, and accuracy

- **RAPL is usually NOT exposed inside a KVM guest**, so on the lab VM
  this reports power via the flat `ENERGY_TDP_WATTS` model (set it to
  your CPU's TDP for a better estimate). For real hardware energy, run on
  **bare metal** Fedora where `/sys/class/powercap` exists.
- The **more accurate** approach uses hardware **performance counters**
  (CPU cycles, instructions, cache misses) read in eBPF via a
  `PERF_EVENT_ARRAY` + `bpf_perf_event_read_value`, fed into a power
  model — what Kepler does on bare metal. That needs a guest **vPMU**
  (enable in libvirt) and is noted as the accuracy upgrade; the
  CPU-time-share model here works everywhere and needs no special
  hardware.

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM: builds, loads, attaches the `sched_switch` tracepoint, and runs
the power-by-process table as described. As expected on a KVM guest, RAPL
(`/sys/class/powercap`) is not exposed, so the VM reports power via the
flat `ENERGY_TDP_WATTS` model — the CPU-time-share *attribution* is
verified, but absolute watts are modeled, not measured; run on bare metal
for real RAPL energy. The `sched_switch` struct offsets (prev_comm@8,
prev_pid@24) can be kernel-version-specific, and per-process energy is an
estimate by construction.
