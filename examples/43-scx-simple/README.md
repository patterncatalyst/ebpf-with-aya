# 43 · sched_ext and struct_ops (scx_simple)

A BPF program can *be* the CPU scheduler. This chapter teaches the
`struct_ops` model through `scx_simple` — the minimal `sched_ext` scheduler —
runs the real one, and attaches an **Aya** probe to watch it.

> **Language note.** Unlike the rest of the book, `sched_ext` schedulers are
> written today with the **BPF callbacks in C** (the upstream `scx` project)
> and **user space in Rust** (`scx_utils`/`libbpf`); Aya's kernel-side
> `struct_ops` support is still emerging. So here we run the shipping
> `scx_simple` and observe it with Aya, rather than rebuilding it.

## Contents

- `reference/scx_simple.bpf.c` — the real scheduler callbacks, for reading
  (ships prebuilt in Fedora's `scx-scheds` as the `scx_simple` binary).
- `scx-watch-ebpf` / `scx-watch` — an Aya `sched:sched_switch` probe that
  counts context switches per CPU and exports `ebpf_ctxsw_total{cpu}`.

## Run it

```bash
./demo.sh          # start scx_simple on $VM + workload + attach the Aya probe
./demo.sh build    # just build the probe on the host
```

Needs the target on **kernel ≥ 6.12** with `sched_ext` (`/sys/kernel/sched_ext`
present); the demo installs `scx-scheds` if needed.

## Verify on the target

```bash
cat /sys/kernel/sched_ext/state         # "enabled" while scx_simple runs
cat /sys/kernel/sched_ext/root/ops      # active scheduler name: simple
sudo bpftool struct_ops list            # the registered sched_ext_ops
sudo pkill -x scx_simple                # revert to the default scheduler
```

## Verification status

**Unverified** — kernel ≥ 6.12. Confirm: `scx-scheds` provides `scx_simple`
and it activates (`/sys/kernel/sched_ext/state` → `enabled`), the
`/sys/kernel/sched_ext/` paths, `bpftool struct_ops list`, and that the Aya
`sched_switch` probe counts switches per CPU while it runs.
