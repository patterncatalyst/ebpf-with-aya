# 44 · A realistic policy: keeping work on warm cores (scx_nest)

`scx_nest` concentrates work on a small set of warm, high-frequency cores
instead of spreading it thin. This chapter explains the policy, runs the real
`scx_nest`, and uses an **Aya** per-CPU busy probe to make the nest visible.

> **Language note** (as in Ch 43): `sched_ext` schedulers are written today
> with BPF callbacks in C and user space in Rust; Aya's kernel-side
> `struct_ops` is emerging. So we run the shipping `scx_nest` and observe it.

## Contents

- `reference/scx_nest.bpf.c` — a *simplified* excerpt of the nest CPU
  selection, for reading (the real scx_nest ships in `scx-scheds`).
- `cpu-busy-ebpf` / `cpu-busy` — an Aya `sched:sched_switch` probe computing
  per-CPU busy time, exported as `ebpf_cpu_busy_ns_total{cpu}` and printed as
  a live per-CPU busy-percent bar.

## Run it

```bash
./demo.sh          # run scx_nest on $VM + a moderate load + attach the probe
./demo.sh build    # just build the probe on the host
```

Needs kernel ≥ 6.12 with `sched_ext`. The load is deliberately *moderate*
(fewer busy tasks than cores) — that's the regime where the nest concentrates
work on a few cores instead of spreading it.

## Verify on the target

```bash
cat /sys/kernel/sched_ext/root/ops     # active scheduler: nest
mpstat -P ALL 2 1                       # a few CPUs busy, the rest idle
grep MHz /proc/cpuinfo                  # busy cores clocked higher
sudo pkill -x scx_nest                  # revert to the default scheduler
```

## Verification status

**Unverified** — kernel ≥ 6.12. Confirm `scx-scheds` provides `scx_nest` and
it activates, the `sched_switch` `prev_pid` offset (24) and `prev_pid == 0`
as idle, and that under moderate load the per-CPU busy series (and `mpstat`)
show concentration on a few cores.
