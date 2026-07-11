# Example 11 — execsnoop (tracepoint on execve, reading argv)

Trace every program launched on the target, with its full command line.

## What this shows (new vs. Ch 9–10)

- Reading **argv** — `const char *const *`, a user pointer to an array
  of user string pointers. The new skill: a **bounded loop** (the
  verifier requires a constant trip count) reading each pointer then
  each string, into **fixed per-arg slots** so there's no dynamic offset
  arithmetic to upset the verifier.
- Writing **directly into the reserved ring-buffer slot** rather than a
  stack buffer — the event is ~800 bytes, well over the 512-byte BPF
  stack limit, so it must not live on the stack.
- Reassembling the command line in user space from the fixed slots.

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # build + deploy to VM + run (Ctrl-C to stop)
```

Launch programs on the target:

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'ls -la /tmp; uname -a; id'
```

A `PID UID COMM CMDLINE` table fills in;
`ebpf_events_total{program="execsnoop"}` climbs in Grafana.

## Cross-check (on the VM)

```bash
[vm]$ sudo execsnoop-bpfcc          # the BCC/C equivalent
[vm]$ sudo bpftrace -e 'tracepoint:syscalls:sys_enter_execve { printf("%s\n", str(args.filename)); }'
```

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches, and
runs as described. The argv loop passes the verifier, the `bpf_probe_read_user`
/ `bpf_probe_read_user_str_bytes` readers work as written, and the ~800-byte
event reserves and fills correctly. Attach targets, the `sys_enter_execve`
struct offsets, and the verifier-acceptable form of the argv loop can be
kernel- and aya-version-specific.
