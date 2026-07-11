# Example 30 — sockops (cgroup socket-operations callbacks)

React to TCP connection lifecycle events with a **`sock_ops`** program
attached to a **cgroup** — a fundamentally different model from
tracepoints and kprobes.

## What this shows (new)

- A **`sock_ops`** program (new type) attached to the **cgroup-v2 root**.
  The TCP stack *calls it back* at socket-lifecycle moments (connect,
  active/passive established, retransmit, RTT, state change) for sockets
  in that cgroup.
- The context **hands you the 4-tuple directly** (`local_ip4`,
  `remote_ip4`, `local_port`, `remote_port`) — no packet or struct
  parsing.
- We emit each established connection with its **direction** (active =
  we connected, passive = we accepted).

## What makes sock_ops special

Beyond observing, `sock_ops` can **act** on the connection: set socket
options, switch congestion control, and enable further callbacks (RTT,
state transitions) via `cb_flags`. It's also how sockets get added to a
**sockmap** for `sk_msg`/`sk_skb` redirection. This example stays on the
observe side; the act side is the chapter's "where to go next."

## Run it (two-VM, needs cgroup-v2 + privileges)

```bash
scripts/lab/provision-vm.sh ebpf-peer    # if needed
./demo.sh
```

```
DIR      LOCAL                  REMOTE
active   10.0.0.21:5155         10.0.0.32:9100
passive  10.0.0.21:9200         10.0.0.32:51777
```

`ebpf_sock_established_total{dir}` in Grafana.

## ⚠ Verification status

**Unverified.** Risks: `SockOps::attach(cgroup_file)` and the
`SockOpsContext` accessors (`op()`, `local_ip4()`, `remote_ip4()`,
`local_port()`, `remote_port()`) in aya 0.14.x; the established-callback
op constants (4/5); `local_port` host-order vs `remote_port` network-order
convention; requires unified cgroup-v2 mounted at `/sys/fs/cgroup`.
Record results in `_plans/reconciliation-plan.md`.
