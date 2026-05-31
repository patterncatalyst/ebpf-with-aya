# 55 · struct_ops: BPF that implements the kernel

struct_ops lets BPF supply a whole **kernel interface** — a struct of function
pointers the kernel calls into. The original user is **TCP congestion
control**; the same mechanism powers sched_ext (Part 6), HID-BPF, bpf Qdisc,
and FUSE.

## Contents

- `reference/cc.bpf.c` — canonical minimal Reno-style CC algorithm
  (`SEC("struct_ops/…")` programs + a `SEC(".struct_ops.link")` struct).
- `illustrative/cc_aya.rs` — where Aya is heading (struct_ops authoring is
  emerging; read-only).
- `demo.sh` — compiles the C and registers it with `bpftool struct_ops`.

## Run it

```bash
./demo.sh          # compile + register on $VM; shows it in tcp_available_congestion_control
```

Needs `clang`, `libbpf-devel`, and `bpftool` on the target (Chapter 4).

## Cross-check

```bash
sysctl net.ipv4.tcp_available_congestion_control   # bpf_reno listed
sudo bpftool struct_ops show
sudo sysctl -w net.ipv4.tcp_congestion_control=bpf_reno && ss -ti | grep bpf_reno
```

## Verification status

**Unverified.** Confirm the algorithm compiles against this kernel's
`vmlinux.h`; that `bpftool struct_ops register` installs it and it appears in
`tcp_available_congestion_control`; the `tcp_sock` field names
(`snd_cwnd`/`snd_ssthresh`/`prior_cwnd`); and treat the aya-ebpf rendering as
emerging.
