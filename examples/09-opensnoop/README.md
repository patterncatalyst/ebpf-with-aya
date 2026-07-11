# Example 09 — opensnoop (tracepoints on openat)

Trace every `openat` on the target: which process opened which file,
with what flags, and whether it succeeded (fd) or failed (`-errno`).

## What this shows (new vs. Ch 7–8)

- Attaching to **tracepoints** (`syscalls:sys_enter_openat`,
  `sys_exit_openat`) instead of a kernel function — *stable* events
  with a documented format, no struct-layout fragility.
- Reading **tracepoint arguments by offset** from the context, where the
  offsets come from the event's format file:
  `cat /sys/kernel/tracing/events/syscalls/sys_enter_openat/format`.
- The **user-vs-kernel memory** distinction: the filename pointer at
  syscall entry is a *user* pointer, so we use
  `bpf_probe_read_user_str_bytes` (contrast Ch 7–8's *kernel* reads).
- Same entry→exit correlation via a `HashMap` keyed by `pid_tgid` as
  Ch 8, here pairing the filename (entry) with the result (exit).

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # build + deploy to VM + run (Ctrl-C to stop)
```

Generate opens on the target (a deliberate miss shows a negative ret):

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'cat /etc/hostname /etc/os-release /nope-$RANDOM 2>/dev/null; true'
```

`ebpf_events_total{program="opensnoop",result="ok|err"}` lands in
Grafana.

## Cross-check (on the VM)

```bash
[vm]$ sudo bpftrace -e 'tracepoint:syscalls:sys_enter_openat { @[comm] = count(); }'
[vm]$ sudo opensnoop-bpfcc        # the BCC/C equivalent, for comparison
```

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on
the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches
both tracepoints, and runs as described. The tracepoint field offsets
(`filename`@24, `flags`@32, exit `ret`@16) are long-stable x86_64 values
but remain kernel-version-specific — verify them against your kernel's
format file, as the chapter shows.
