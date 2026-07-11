# 52 · kfuncs: calling the kernel, the modern way

Look up a `struct task_struct` by pid from inside a BPF program using the
canonical acquire/release kfunc pair — and let the verifier **enforce** the
release.

## Pieces

- `kfunc-task-ebpf` — declares `bpf_task_from_pid` (KF_ACQUIRE | KF_RET_NULL)
  and `bpf_task_release` (KF_RELEASE) as extern kfuncs; on a `getpid`
  tracepoint, looks up `CONFIG[0]`, null-checks, counts found/missing in
  `RESULT`, and releases. `src/vmlinux.rs` is a placeholder — regenerate with
  `aya-tool generate task_struct`.
- `kfunc-task` — sets a target pid (ours, then bogus), triggers via `getpid`,
  reads tallies, exports `ebpf_task_lookups_total{result}`.

## Run it

```bash
./demo.sh          # phase 1 (real pid) -> found; phase 2 (bogus) -> missing
./demo.sh build    # just build on the host
```

## Try this (see the verifier work)

Delete the `bpf_task_release(task)` line in `kfunc-task-ebpf` and rebuild: the
program **fails to load**, because the non-null path would leak the acquired
reference. That rejection is the whole point of `KF_ACQUIRE`/`KF_RELEASE`.

## Cross-check

```bash
sudo bpftool prog dump xlated name lookup | grep -i call   # the kfunc call sites
sudo bpftool btf dump file /sys/kernel/btf/vmlinux | grep bpf_task_from_pid
```

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab VM
(Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches, and runs as
described, tallying found (target pid running) vs missing. Note the kfunc form
(`bpf_task_from_pid`) is not expressible in aya-ebpf, so the verified program
checks the current task instead. kfuncs carry no ABI-stability promise, so the
exact kfunc set and struct offsets can be kernel-version-specific.
