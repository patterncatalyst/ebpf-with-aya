---
title: "sockops"
order: 30
part: Networking
description: "Meet a different model: a sock_ops program attached to a cgroup that the TCP stack calls back at connection-lifecycle moments, handing you the 4-tuple directly — and that can also act on the socket, not just observe it."
duration: 30 minutes
---

Every networking tool so far *observed* — a tracepoint fired, a packet
arrived, and you read it. `sock_ops` is different in two ways: it's
attached to a **cgroup** rather than a function or interface, and the
TCP stack **calls it back** at lifecycle moments — and it can **act**,
not just watch. This chapter uses it to track established connections,
and points at the things only `sock_ops` can do.

The code is in `examples/30-sockops/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

## A callback, scoped to a cgroup

You attach a `sock_ops` program to a **cgroup-v2** directory, and from
then on the TCP stack invokes it for sockets in that cgroup at defined
moments — connect, active/passive established, retransmit, RTT update,
state change:

{% include excalidraw.html
   file="sockops-cb"
   alt="A sock_ops program attached to a cgroup is called back by the TCP stack at lifecycle moments: connect, active/passive established, RTT and state-change callbacks, and it can also act by setting socket options, congestion control, and callback flags."
   caption="Figure 30.1 — sock_ops: the stack calls back into your program" %}

The program switches on `ctx.op()` to see *which* moment fired. We react
to the two "established" callbacks and emit the connection's direction
and 4-tuple — which the context hands us **directly**, no packet or
struct parsing:

```rust
#[sock_ops]
pub fn track(ctx: SockOpsContext) -> u32 {
    let dir = match ctx.op() {
        BPF_SOCK_OPS_ACTIVE_ESTABLISHED_CB  => DIR_ACTIVE,   // we connected
        BPF_SOCK_OPS_PASSIVE_ESTABLISHED_CB => DIR_PASSIVE,  // we accepted
        _ => return 0,                                       // ignore other callbacks
    };
    if let Some(mut slot) = EVENTS.reserve::<SockEvent>(0) {
        let ev = SockEvent {
            local_ip4:  ctx.local_ip4(),  remote_ip4:  ctx.remote_ip4(),
            local_port: ctx.local_port() as u16, remote_port: ctx.remote_port() as u16,
            dir,
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    0
}
```

`ctx.op()` is the dispatch: one program is invoked for *every* lifecycle
moment, and the `match` picks out the two we care about, returning early
for the rest. The accessors (`ctx.local_ip4()`, `ctx.remote_port()`, …)
read the connection's 4-tuple straight from the `bpf_sock_ops` context
the kernel passes in — no packet to parse, no struct offset to chase.

The cgroup scoping is the point: attach to a container's cgroup and you
observe exactly that workload's connections, no PID or tuple filtering
needed.

## Attaching to the cgroup

A `sock_ops` program attaches to a **cgroup-v2 directory**, not a
function or an interface. User space opens the cgroup as a file and
passes it to `attach`:

```rust
let cgroup = std::fs::File::open("/sys/fs/cgroup")?;   // the unified v2 root
let prog: &mut SockOps = ebpf.program_mut("track").unwrap().try_into()?;
prog.load()?;
prog.attach(cgroup)?;
```

Attaching at the root covers every socket on the box; point `attach` at
a container's cgroup directory instead and the same program watches
exactly that workload — the scoping the diagram promises, with no
in-program filtering. Draining is the usual `RingBuf` loop: decode each
`SockEvent`, format the local/remote endpoints, and count by direction
(`ebpf_sock_established_total{dir}`).

## It can act, not just observe

What sets `sock_ops` apart from a tracepoint is that it runs *inside*
the stack's decision points and can change them:

- **Set socket options** at established time (send/receive buffer sizes,
  TCP options) — per-cgroup tuning without touching the app.
- **Switch congestion control** for matching connections.
- **Enable more callbacks** via `cb_flags` — opt into RTT
  (`BPF_SOCK_OPS_RTT_CB`) or state-change callbacks to build, say, a
  per-cgroup RTT histogram.
- **Populate a sockmap** so `sk_msg`/`sk_skb` programs can redirect
  traffic between sockets in-kernel (the basis of service-mesh
  acceleration).

This example stays on the observe side; those are the directions to take
it. (`sk_msg`/sockmap redirection is its own larger topic.)

## A note on byte order

Small but real: in the `sock_ops` context, `local_port` is in **host**
byte order while `remote_port` is in **network** byte order — a kernel
convention that bites everyone once. The user space converts
accordingly.

## Build, deploy, observe

`sock_ops` needs unified **cgroup-v2** (Fedora's default) and privilege:

```bash
scripts/lab/provision-vm.sh ebpf-peer    # if needed
cd examples/30-sockops && ./demo.sh
```

The demo opens connections in both directions between target and peer:

```text
DIR      LOCAL                  REMOTE
active   10.0.0.21:5155         10.0.0.32:9100
passive  10.0.0.21:9200         10.0.0.32:51777
```

`ebpf_sock_established_total{dir}` in Grafana — active vs. passive
connection rates for the cgroup.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `rate(ebpf_sock_established_total[1m])` — newly established connections over time.

## Cross-check

```bash
[vm]$ sudo bpftool cgroup tree           # see sock_ops attached to the cgroup
[vm]$ ss -tan                            # established sockets, to compare
```

## What you learned

- `sock_ops` attaches to a **cgroup** and is a **callback** the TCP
  stack invokes at lifecycle moments (`ctx.op()` says which).
- The context provides the **4-tuple directly** — no parsing.
- Uniquely, it can **act**: set socket options, congestion control,
  `cb_flags`, and feed sockmaps — not just observe.

That wraps the connection/socket section. Next: **`tc`** traffic control
and the first **XDP** program — moving out to the edge of the stack.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Risks: `SockOps::attach(cgroup)` and the `SockOpsContext` accessors in
aya 0.13.x; the established-callback op constants; the
`local_port`/`remote_port` byte-order convention; requires cgroup-v2 at
`/sys/fs/cgroup`. The first build and run are the test.*
