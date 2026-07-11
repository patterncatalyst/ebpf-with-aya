# 57 · BPF iterators: walking the kernel

A BPF iterator is the body of an in-kernel loop: the kernel walks a set (every
task, socket, or map element) and calls your program **once per element**,
which reads fields directly and emits through a **seq_file** you `cat`.

## Contents

- `reference/task_iter.bpf.c` — canonical `SEC("iter/task")` program: header on
  the first call, one row per task, summary at the end.
- `illustrative/iter_aya.rs` — where Aya is heading (iterator support emerging;
  read-only).
- `demo.sh` — compiles it and pins it with `bpftool iter pin`, then `cat`s the
  result.

## Run it

```bash
./demo.sh          # compile + pin + cat a BPF-built process table on $VM
```

Needs `clang`, `libbpf-devel`, `bpftool` on the target.

## Cross-check

```bash
sudo cat /sys/fs/bpf/task_iter | wc -l    # rows produced
ps -e | wc -l                              # compare to the real task count
sudo bpftool iter help                     # available iterator targets
```

## Open-coded iterators

For bounded loops inside any program: `bpf_for(i, 0, n)`, `bpf_repeat(n)`, and
the `bpf_iter_*` (`KF_ITER_*`) kfuncs from Chapter 52.

## Verification status

**Verified — Fedora 44, kernel 7.1.3 (clang 22, bpftool v7.6.0).** Compiles
against this kernel's `vmlinux.h`; `bpftool iter pin` + `cat` produce the
process table (469 tasks) with `tgid`/`pid`/`comm`. aya iterator support is
emerging; the C path is canonical.
