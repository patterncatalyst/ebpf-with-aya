# Example 12 — exitsnoop (tracepoint on exit_group)

The bookend to execsnoop: trace process termination and its exit code.

## What this shows

- Capturing the **exit code without touching `task_struct`**. We attach
  to `syscalls:sys_enter_exit_group` — the syscall every normal process
  exit funnels through — whose single argument *is* the exit status.
  That keeps the program robust across kernels, unlike the libbpf
  `exitsnoop`, which reads `exit_code`/start-time fields out of
  `task_struct` via CO-RE.
- A subtle but important decode: the `exit_group` argument is the
  **raw** status (e.g. `exit(3)` → `3`), so the exit code is its low 8
  bits — *different* from `task_struct->exit_code`, which packs the code
  in the high byte and a terminating signal in the low byte.
- Labelling the metric `status="ok|nonzero"` so failed exits stand out
  in Grafana.

## execsnoop + exitsnoop together

Run both (in two terminals, or as separate deploys) and you bracket
every process lifetime: execsnoop logs the launch and command line,
exitsnoop logs the termination and code. That pairing is the basis of
process-accounting and short-lived-process detection.

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # build + deploy to VM + run (Ctrl-C to stop)
```

Generate exits with various codes on the target:

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" '(true); (false); sh -c "exit 3"'
```

You'll see codes `0`, `1`, and `3`;
`ebpf_events_total{program="exitsnoop",status=...}` splits ok vs.
nonzero in Grafana.

## Cross-check (on the VM)

```bash
[vm]$ sudo bpftrace -e 'tracepoint:syscalls:sys_enter_exit_group { printf("%s code %d\n", comm, args.error_code); }'
```

## ⚠ Verification status

**Unverified.** Confirm the `error_code` offset (@16) against the
format file, the `read_at`/attach API, and the exit-code decode
(`& 0xff`) against a known `exit(N)`. A process killed by a *signal*
doesn't call `exit_group`, so it won't appear here — that's expected;
catching signal-deaths is a `sched:sched_process_exit` extension.
Record results in `_plans/reconciliation-plan.md`.
