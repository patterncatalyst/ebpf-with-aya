# 42 · A security sensor: telemetry and shielding

The sensor layer of a runtime-security tool, in miniature: three
attacker-relevant tracepoints (`execve`, `ptrace`, `setuid`) emit one uniform
`SecEvent` stream over a single `RingBuf`. The user side classifies each by
type and severity and ships it to Grafana.

## What it does

- `on_exec` / `on_ptrace` / `on_setuid` (tracepoints on `sys_enter_execve` /
  `sys_enter_ptrace` / `sys_enter_setuid`) each `emit` a `SecEvent`
  (type, pid, comm) — kernel side kept minimal.
- The loader reads the stream, tags severity (`exec`=info, `ptrace`/`setuid`
  =warning), prints classified lines, and exports
  `ebpf_sec_events_total{type,severity}`.
- Observe-only; the chapter shows how to pair it with an LSM hook to *shield*.

## Run it

```bash
./demo.sh          # build + deploy + exercise exec/ptrace/setuid
./demo.sh build    # just build on the host
```

## Verify on the target

```bash
sudo bpftool prog show | grep tracepoint     # the three sensor programs
strace -p $(pgrep -n sleep) -e trace=none &   # a ptrace event appears instantly
id; sudo -u nobody id                          # exec + setuid events
```

## Verification status

**Unverified** — confirm the tracepoint names (`sys_enter_execve`,
`sys_enter_ptrace`, `sys_enter_setuid`; a kernel may expose `setreuid`/
`setresuid` instead), `RingBuf` shared across three programs, and that the
classified counts track the exercised operations.
