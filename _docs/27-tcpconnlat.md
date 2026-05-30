---
title: "tcpconnlat"
order: 27
part: Networking
description: "Open Part 5 by building out the second VM and measuring active TCP connection latency — connect() to SYN-ACK — with kprobes on the kernel TCP stack keyed by the socket pointer, reading struct fields by offset."
duration: 35 minutes
---

Welcome to **Networking**, the part where the lab grows its second
machine. Everything so far ran on one guest; from here on we need real
host-to-host traffic, so we bring up **ebpf-peer** alongside
**ebpf-target** and start watching connections cross between them. The
first tool, `tcpconnlat`, answers "how long do my outbound connections
take to establish?" — and introduces probing the **kernel TCP stack**.

The code is in `examples/27-tcpconnlat/`.

## Where eBPF sits in the network path

Before the specifics, the lay of the land. eBPF can attach at many
points along a packet's journey, and the whole of Part 5 is a tour of
them — from the earliest (XDP, in the driver) to the latest (socket
operations, up by the application):

{% include excalidraw.html
   file="net-hooks"
   alt="The network path from NIC driver through XDP, tc/tcx, the IP/TCP stack, to the socket and application, showing where each eBPF program type attaches: XDP and tc/tcx before the stack for drop/redirect/rewrite, kprobes in the TCP stack, and sockops/filters at the socket."
   caption="Figure 27.1 — where eBPF attaches along the network path" %}

This chapter lives in the **stack** layer: kprobes on the kernel's TCP
functions. Later chapters move outward to `tc`/`tcx` and XDP.

## Bring up the peer

The connection-latency measurement needs something to connect *to*. Our
provisioning script already takes a guest name, so the peer is one
command:

```bash
scripts/lab/provision-vm.sh ebpf-peer
```

Both guests share the libvirt NAT network, so they reach each other by
IP. `scripts/lab/lab-ips.sh` prints both. `tcpconnlat` runs on
**ebpf-target**; the demo drives connects to a listener on
**ebpf-peer**.

## Timing the handshake

An active (outbound) connection's latency is the time from `connect()`
— which sends the SYN — until the SYN-ACK comes back and the socket is
established. Two kprobes bracket that window:

{% include excalidraw.html
   file="tcp-handshake"
   alt="A kprobe on tcp_v4_connect stamps t0 keyed by the struct sock pointer when the client sends SYN; the SYN-ACK returns from the server; a kprobe on tcp_rcv_state_process computes the delta and emits the connection latency."
   caption="Figure 27.2 — connection latency across two kprobes" %}

```rust
#[kprobe] pub fn tcp_v4_connect(ctx) {           // SYN sent
    let sk = ctx.arg(0)?;                         // struct sock *
    START.insert(&sk, &ConnStart { ts: ktime(), .. });
}
#[kprobe] pub fn tcp_rcv_state_process(ctx) {     // SYN-ACK processed
    let sk = ctx.arg(0)?;
    let start = START.get(&sk)?; START.remove(&sk);
    emit(now - start.ts);
}
```

The new idea is the **key**. Kernel-side, we don't have a `pid_tgid` to
correlate on — we have the **`struct sock *`** pointer, which is the
same for both probes of one connection. It's the kernel analogue of the
entry/exit key you've used since Chapter 8. The first time our sk shows
up in `tcp_rcv_state_process` is essentially the SYN-ACK arriving, so we
compute the latency there and forget the socket.

## Reading kernel struct fields (the fragile part)

To report *which* connection, we read the destination from the socket.
The address and port live at the very head of `struct sock` (inside
`sock_common`), and we copy them with `bpf_probe_read_kernel`:

```rust
let daddr = bpf_probe_read_kernel((sk + SKC_DADDR) as *const u32)?;  // offset 0
let dport = bpf_probe_read_kernel((sk + SKC_DPORT) as *const u16)?;  // offset 12
```

Those offsets are the soft spot. They're at the head of the struct,
which is fairly stable, but "fairly" isn't "guaranteed" — struct layouts
shift between kernel versions, and hardcoding offsets is exactly the
fragility kprobes are infamous for. The real fix is **CO-RE**, which
relocates field offsets at load time against the running kernel's BTF so
the same binary works everywhere; that's the deep-dive in Chapter 56.
For now, verify the offsets with `pahole -C sock_common` and treat them
as provisional. (Chapter 28 sidesteps the whole problem with a
tracepoint that hands you the fields directly — a pointed contrast.)

## Build, deploy, observe

```bash
cd examples/27-tcpconnlat && ./demo.sh
```

The demo starts a listener on the peer, drives `curl` connects from the
target, and runs `tcpconnlat`:

```text
PID      COMM             DEST                   LAT(ms)
4821     curl             10.0.0.32:8080         0.412
```

`tcp_connect_latency_ms` in Grafana gives you a connection-latency
histogram — watch it climb if you add latency to the link (`tc qdelay`,
or a busy peer).

## Cross-check

```bash
[vm]$ sudo tcpconnlat-bpfcc
[vm]$ sudo pahole -C sock_common /sys/kernel/btf/vmlinux | grep -E 'skc_daddr|skc_dport'
```

## What you learned

- Part 5 needs **two VMs**; the peer is `provision-vm.sh ebpf-peer`.
- eBPF attaches all along the network path (Figure 27.1); this chapter
  is in the **TCP stack** via kprobes.
- Correlate the two probes of one connection by the **`struct sock *`**
  pointer — the kernel-side entry/exit key.
- Reading kernel struct fields by **offset** works but is fragile;
  **CO-RE** (Ch 56) is the portable fix.

Next: **`tcpstates`** traces the whole TCP state machine — with a single
tracepoint that needs none of this offset chasing. See the
[roadmap]({{ "/plans/iteration-plan/" | relative_url }}).

---

*Verification status: <span class="status status--unverified">unverified</span>.
Highest-risk: the `sock_common` offsets (`skc_daddr`@0, `skc_dport`@12 —
CO-RE removes the guesswork); the first-`tcp_rcv_state_process`≈SYN-ACK
assumption; `KProbe::attach` to these symbols in aya 0.13.x; IPv4 only.
The first build and run are the test.*
