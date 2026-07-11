# Example 19 — trace goroutine states (uprobe on runtime.casgstatus)

Watch the Go scheduler's state machine: every goroutine transition
(runnable → running → waiting → …) flows through one runtime function,
and a uprobe on it sees them all.

## What this shows (Go-specific)

- Probing `runtime.casgstatus(gp, oldval, newval)` — called on **every**
  goroutine state change.
- **Go's register ABI** (1.17+): args are in RAX, RBX, RCX, … not the C
  ABI registers. So `newval` (3rd arg) is in **RCX**, and we read it from
  `pt_regs` directly — `ctx.arg(2)` would read the wrong register.
- **No uretprobes on Go**: Go moves/grows goroutine stacks; uretprobe
  return trampolines can corrupt them and crash the process. Uprobes
  (entry) only.
- PID is the **OS thread (M)**, not the goroutine — Go multiplexes many
  goroutines onto few threads.

## Requires the Go toolchain (host)

```bash
sudo dnf install -y golang
```

## Run it

```bash
./demo.sh build     # build Go target + tracer on the host
./demo.sh           # ship Go target to the VM, run it, attach the uprobe
```

You'll see a stream of transitions; `ebpf_events_total{program="goroutine",state=...}`
breaks them down by state in Grafana (a great view of scheduler
pressure — lots of `waiting` ↔ `runnable` churn means contention).

## Cross-check (on the VM)

```bash
[vm]$ nm /home/fedora/target-go | grep runtime.casgstatus
[vm]$ sudo bpftrace -e 'uprobe:/home/fedora/target-go:runtime.casgstatus { @[reg("cx")] = count(); }'
```

## ⚠ Verification status

**Unverified.** Highest-risk: the **Go register ABI** read (RCX for
`newval`) and the `pt_regs` field name (`rcx`) in aya 0.14.x's bindings;
the `runtime.casgstatus` symbol being present (Go embeds symbols by
default, but `-ldflags=-s` strips them); and the goroutine state value
mapping for your Go version. Confirm the register with the `bpftrace`
`reg("cx")` line. Record results in `_plans/reconciliation-plan.md`.
