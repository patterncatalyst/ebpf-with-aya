# Example 28 — tcpstates (TCP state transitions)

Watch every TCP connection move through the state machine —
`SYN_SENT → ESTABLISHED → … → CLOSE` — using a single stable tracepoint.

## What this shows (and the contrast with Ch 27)

- **`sock:inet_sock_set_state`** fires on every TCP state change and
  carries the **old/new state, addresses, and ports directly**. No
  kprobes, no `struct sock` offset chasing — the deliberate, clean
  counterpart to Chapter 27's fragile-by-nature kprobe approach.
- Reading well-defined **tracepoint fields by offset** (from the format
  file) and mapping TCP state numbers to names.

## Run it (two-VM)

```bash
scripts/lab/provision-vm.sh ebpf-peer    # if not already up
./demo.sh
```

```
SRC                    DST                    OLD           -> NEW
10.0.0.21:0            10.0.0.32:8080         CLOSE         -> SYN_SENT
10.0.0.21:53344        10.0.0.32:8080         SYN_SENT      -> ESTABLISHED
10.0.0.21:53344        10.0.0.32:8080         ESTABLISHED   -> CLOSE
```

`ebpf_tcp_state_transitions_total{newstate}` in Grafana.

## Cross-check (on the target VM)

```bash
[vm]$ sudo tcpstates-bpfcc
[vm]$ cat /sys/kernel/tracing/events/sock/inet_sock_set_state/format   # confirm offsets
```

## ⚠ Verification status

**Unverified.** Risks: the tracepoint field offsets
(oldstate@16/newstate@20/sport@24/dport@26/protocol@30/saddr@32/daddr@36 —
verify vs. the format file) and the sport/dport byte order as stored by
this tracepoint; IPv4 fields shown (v6 fields exist further in). Record
results in `_plans/reconciliation-plan.md`.
