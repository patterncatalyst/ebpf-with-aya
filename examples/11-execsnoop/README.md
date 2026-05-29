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

**Unverified.** Highest-risk items on real hardware:

1. **The argv loop.** Reading an array of user pointers in a
   verifier-acceptable bounded loop is the part most likely to need
   adjustment — if the verifier rejects it, compare against the Aya
   examples repo's execsnoop and adjust the bound/masking. `MAX_ARGS=8`,
   `ARG_LEN=64` keep it conservative.
2. `bpf_probe_read_user` (single value) and `bpf_probe_read_user_str_bytes`
   API/signatures in aya 0.13.x.
3. `sys_enter_execve` offsets (filename@16, argv@24).
4. Event size (~800 B) written into the ring slot — confirm `reserve`
   handles it.

Record results in `_plans/reconciliation-plan.md`.
