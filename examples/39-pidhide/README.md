# 39 · Hiding a process (lab-only offense)

**LAB-ONLY.** An eBPF "pidhide": rewrite the `getdents64` result buffer so a
chosen PID disappears from `/proc` — invisible to `ps`, `top`, `ls /proc` —
while the process keeps running. Built to understand rootkit technique *and*
its detection; it uses the kernel-tainting `bpf_probe_write_user`.

## What it does

- `enter_getdents` (tracepoint on `sys_enter_getdents64`) stashes the user
  buffer pointer by `pid_tgid`.
- `exit_getdents` (tracepoint on `sys_exit_getdents64`) walks the
  `linux_dirent64` records and, when it finds the target PID's entry,
  extends the **previous** record's `d_reclen` to swallow it
  (`bpf_probe_write_user`).
- Exports `ebpf_proc_hidden_total`.

## Run it

```bash
./demo.sh          # build + deploy + start a sleep + hide it + watch /tmp/pidhide.log
./demo.sh build    # just build on the host
HIDE_PID=1234 ...  # hide a specific pid
```

## Detecting it (the point)

- `cat /proc/sys/kernel/tainted` is non-zero; `dmesg` warns about
  `bpf_probe_write_user`.
- `sudo bpftool prog show` lists tracepoint programs on `getdents64`.
- The PID still exists everywhere that doesn't go through `getdents64`:
  `kill -0 <pid>` works, `/proc/<pid>/` stats fine, and a BPF task iterator
  shows it.

## Verification status

**Unverified** — confirm: the `getdents64` enter/exit offsets (dirent @24,
ret @16), `linux_dirent64` field offsets (`d_reclen` @16, `d_name` @19), that
`bpf_probe_write_user` is permitted and hides the PID, the bounded walk
passing the verifier, and that `/proc/sys/kernel/tainted` flips.
