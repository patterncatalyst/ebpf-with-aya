# 39 · Hiding a process (lab-only offense)

**LAB-ONLY.** An eBPF "pidhide": rewrite the `getdents64` result buffer so a
chosen PID disappears from `/proc` — invisible to `ps`, `top`, `ls /proc` —
while the process keeps running. Built to understand rootkit technique *and*
its detection; it uses the lab-only `bpf_probe_write_user`.

## What it does

- `enter_getdents` (tracepoint on `sys_enter_getdents64`) stashes the user
  buffer pointer by `pid_tgid`.
- `exit_getdents` (tracepoint on `sys_exit_getdents64`) walks the
  `linux_dirent64` records (up to 512 — enough to cover a whole `/proc` in one
  call) and, when it finds the target PID's entry, extends the **previous**
  record's `d_reclen` to swallow it (`bpf_probe_write_user`).
- Exports `ebpf_proc_hidden_total`.

## Run it

```bash
./demo.sh          # build + deploy + start a sleep + hide it + watch /tmp/pidhide.log
./demo.sh build    # just build on the host
HIDE_PID=1234 ...  # hide a specific pid
```

## Detecting it (the point)

- `sudo bpftool prog show` lists tracepoint programs on `getdents64` — the
  most reliable tell.
- The PID still exists everywhere that doesn't go through `getdents64`:
  `kill -0 <pid>` works, `/proc/<pid>/` stats fine, and a BPF task iterator
  shows it.
- **Note:** on kernel 7.1.3, `bpf_probe_write_user` does **not** taint the
  kernel or emit a `dmesg` warning (verified: `/proc/sys/kernel/tainted` stays
  `0`) — don't rely on a passive taint flag.

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host, run on the lab VM
and confirmed behaviorally: with the target attached, a bare `ls /proc` no
longer lists the PID (hide count increments) while `kill -0 <pid>` succeeds and
`/proc/<pid>/` stats fine; on detach it reappears. The record-walk bound was
raised from 64 to 512 — `ls` returns all 200+ `/proc` entries in one
`getdents64`, so a 64-entry cap scanned only the first slice and hid nothing.
