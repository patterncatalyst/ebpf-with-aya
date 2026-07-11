# Example 10 — sigsnoop (tracepoint on kill)

Trace signals sent via `kill(2)`: who sent which signal to whom.

## What this shows

- The **minimal single-tracepoint** pattern — no entry/exit correlation,
  one event per `kill()`. Simpler than opensnoop (Ch 9), to show the
  floor of per-event tooling.
- Reading two tracepoint args by offset (`pid`@16, `sig`@24 from
  `syscalls:sys_enter_kill`).
- Mapping the raw signal number to a name (`SIGTERM`, `SIGKILL`, …) in
  **user space** — a reminder that the kernel program stays tiny and
  dumb while user space does the friendly formatting.
- Exporting `ebpf_events_total{program="sigsnoop",signal=NAME}` so you
  can chart signal traffic by type in Grafana.

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # build + deploy to VM + run (Ctrl-C to stop)
```

Generate signals on the target:

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'sleep 60 & p=$!; kill -TERM $p; true'
```

## Cross-check (on the VM)

```bash
[vm]$ sudo bpftrace -e 'tracepoint:syscalls:sys_enter_kill { printf("%s -> pid %d sig %d\n", comm, args.pid, args.sig); }'
```

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on
the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches,
and runs as described. The `sys_enter_kill` offsets and the
`TracePointContext::read_at` / `TracePoint` attach API can be
kernel-version-specific — confirm the offsets against your kernel's
format file if you port this. Note `kill -0` and signals to
already-dead PIDs still generate a `kill()` syscall, so they show up
too.
