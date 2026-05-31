# 61 · Power and efficiency: attributing energy with eBPF

The OS bills CPU **time**, not **joules**. RAPL (via powercap) measures
**socket-level** energy; eBPF supplies the per-workload **shares** to divide it
by: `energy × cpu_time(workload) / cpu_time(total)`.

## Pieces

- `power-ebpf` — `sched_switch` accumulator: per-command on-CPU nanoseconds.
- `power-common` — the `Comm` bucket key.
- `power` — reads the shares; if `/sys/class/powercap/intel-rapl:0/energy_uj`
  exists, estimates watts per command; otherwise reports CPU-time shares.
  Exports `ebpf_oncpu_seconds_total` and (bare metal) `ebpf_estimated_watts`.

## Run it

```bash
./demo.sh          # watts per command on bare metal; CPU-time shares in a VM
./demo.sh build
```

## Honest limits

Time-proportional attribution ignores **DVFS frequency** and **C-states**, so
it's **comparative, not absolute**. RAPL is usually **absent in VMs**. Kepler
(CNCF) is the production exporter (rewritten at 0.10 toward measured sources);
`cpufreq_ext` (struct_ops) is the emerging *control* frontier.

## Cross-check

```bash
cat /sys/class/powercap/intel-rapl:0/energy_uj      # raw socket counter (bare metal)
sudo turbostat --interval 1                          # independent package watts
perf stat -a -e power/energy-pkg/ sleep 1
```

## Verification status

**Unverified.** Confirm the `sched_switch` field offsets; that per-command
on-CPU time accumulates; that the loader reads RAPL where present and falls back
cleanly (expect absent in the VM); compare watts against `turbostat`/`perf`.
