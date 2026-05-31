---
title: "tcpconnlat"
order: 27
part: Networking
description: "Open Part 4 by building out the second VM and measuring active TCP connection latency — connect() to SYN-ACK — with kprobes on the kernel TCP stack keyed by the socket pointer, reading struct fields by offset."
duration: 35 minutes
---

Welcome to **Networking**, the part where the lab grows its second
machine. Everything so far ran on one guest; from here on we need real
host-to-host traffic, so we bring up **ebpf-peer** alongside
**ebpf-target** and start watching connections cross between them. The
first tool, `tcpconnlat`, answers "how long do my outbound connections
take to establish?" — and introduces probing the **kernel TCP stack**.

The code is in `examples/27-tcpconnlat/`.

> **Before you start — this part needs two VMs.** Everything through
> Performance ran on a single guest; Networking needs a second one to
> generate real host-to-host traffic. Confirm your environment is ready
> before running anything:
>
> - **Observability stack up** — `http://127.0.0.1:3000` (Grafana) loads.
>   That's the Chapter 3 stack (`examples/03-observability-stack`).
> - **Target guest running** — `scripts/lab/vm-ip.sh ebpf-target` prints
>   an IP. That's the Chapter 2 guest.
> - **Peer guest provisioned** — bring it up once with
>   `scripts/lab/provision-vm.sh ebpf-peer`, then
>   `scripts/lab/lab-ips.sh` prints both IPs and confirms they're
>   reachable.
>
> If any of those isn't true, set it up first — the networking demos
> can't run without all three. Full setup details (networking
> requirements, resource sizing) are in
> [Chapter 2]({{ "/docs/02-lab-setup/" | relative_url }}).
>
> Then this chapter runs like every other: `cd examples/27-tcpconnlat`,
> read its `README.md`, and `./demo.sh` builds on the host, deploys to
> the target, and runs it (`./demo.sh build` just builds).

## Where eBPF sits in the network path

Before the specifics, the lay of the land. eBPF can attach at many
points along a packet's journey, and the whole of Part 4 is a tour of
them — from the earliest (XDP, in the driver) to the latest (socket
operations, up by the application):

{% include excalidraw.html
   file="net-hooks"
   alt="The network path from NIC driver through XDP, tc/tcx, the IP/TCP stack, to the socket and application, showing where each eBPF program type attaches: XDP and tc/tcx before the stack for drop/redirect/rewrite, kprobes in the TCP stack, and sockops/filters at the socket."
   caption="Figure 27.1 — where eBPF attaches along the network path" %}

This chapter lives in the **stack** layer: kprobes on the kernel's TCP
functions. Later chapters move outward to `tc`/`tcx` and XDP.

## The test topology

`tcpconnlat` runs on **ebpf-target**; the demo starts a listener on
**ebpf-peer** and drives connects to it across the libvirt NAT network
the two guests share. So you watch the target's kernel time its own
outbound connections to the peer — the simplest two-host setup, and the
shape every networking chapter reuses.

## Timing the handshake

An active (outbound) connection's latency is the time from `connect()`
— which sends the SYN — until the SYN-ACK comes back and the socket is
established. Two kprobes bracket that window:

{% include excalidraw.html
   file="tcp-handshake"
   alt="A kprobe on tcp_v4_connect stamps t0 keyed by the struct sock pointer when the client sends SYN; the SYN-ACK returns from the server; a kprobe on tcp_rcv_state_process computes the delta and emits the connection latency."
   caption="Figure 27.2 — connection latency across two kprobes" %}

```rust
// connect(): the SYN is going out. Stamp the start keyed by the sock
// pointer, and grab the destination from the head of struct sock.
#[kprobe]
pub fn tcp_v4_connect(ctx: ProbeContext) -> u32 {
    let sk: u64 = match ctx.arg(0) { Some(p) => p, None => return 0 };
    let daddr = unsafe { bpf_probe_read_kernel((sk as usize + SKC_DADDR) as *const u32) }.unwrap_or(0);
    let dport = unsafe { bpf_probe_read_kernel((sk as usize + SKC_DPORT) as *const u16) }.unwrap_or(0);
    let start = ConnStart {
        ts: unsafe { bpf_ktime_get_ns() },
        pid: (bpf_get_current_pid_tgid() >> 32) as u32,
        daddr, dport, comm: bpf_get_current_comm().unwrap_or_default(),
    };
    let _ = START.insert(&sk, &start, 0);
    0
}

// tcp_rcv_state_process(): the SYN-ACK is being handled. Pair, compute, emit.
#[kprobe]
pub fn tcp_rcv_state_process(ctx: ProbeContext) -> u32 {
    let sk: u64 = match ctx.arg(0) { Some(p) => p, None => return 0 };
    let start = match unsafe { START.get(&sk) } { Some(s) => *s, None => return 0 };
    let _ = START.remove(&sk);                       // first hit ≈ SYN-ACK; don't refire
    let lat_ns = unsafe { bpf_ktime_get_ns() }.saturating_sub(start.ts);
    if let Some(mut slot) = EVENTS.reserve::<ConnEvent>(0) {
        let ev = ConnEvent { pid: start.pid, daddr: start.daddr,
                             dport: start.dport, lat_ns, comm: start.comm };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    0
}
```

The new idea is the **key**. Kernel-side, we have no `pid_tgid` to
correlate on — `tcp_rcv_state_process` runs in softirq context, not in
the connecting task. What we *do* have is `ctx.arg(0)`, the
**`struct sock *`** pointer, which is identical for both probes of one
connection. So the sock pointer is the kernel analogue of the
entry/exit key you've used since Chapter 8: stash under it at connect,
look it up at the SYN-ACK. The first time a sock we stamped reappears in
`tcp_rcv_state_process` is essentially the SYN-ACK being processed, so we
compute the latency there, `remove` the entry so a later call can't
double-count, and emit.

## Reading kernel struct fields (the fragile part)

Those two lines in the connect handler — reading `daddr` and `dport`
from `sk` at `SKC_DADDR` (0) and `SKC_DPORT` (12) — are the soft spot.
The address and port live at the very head of `struct sock` (inside
`sock_common`). `bpf_probe_read_kernel` is mandatory here for the same
reason as Chapter 8: you cannot dereference a kernel pointer directly,
you copy through the helper. The fragility isn't the *read*, it's the
hardcoded **offsets**. They sit at the head of the struct, which is
*fairly* stable — but "fairly" isn't "guaranteed," and hardcoding
offsets is exactly the brittleness kprobes are infamous for. The real
fix is **CO-RE**, which relocates field offsets at load time against the
running kernel's BTF so the same binary works everywhere; that's the
deep-dive in Part 9. For now, verify the offsets with
`pahole -C sock_common` and treat them as provisional. (Chapter 28
sidesteps the problem entirely with a tracepoint that hands you the
fields — a pointed contrast.)

## The user side

Both kprobes attach by kernel-function name, and we drain the shared
ring into an OTLP histogram:

```rust
for name in ["tcp_v4_connect", "tcp_rcv_state_process"] {
    let p: &mut KProbe = ebpf.program_mut(name).unwrap().try_into()?;
    p.load()?;                 // verifier gate
    p.attach(name, 0)?;        // 0 = attach at function entry
}
let hist = meter.f64_histogram("tcp_connect_latency_ms").build();
let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;
```

`attach(name, 0)` is the kprobe form you met in Chapter 7 — the function
name plus an entry offset of `0` — just applied to two TCP-stack
functions instead of one. In the drain loop, each `ConnEvent` becomes a
console row and a histogram sample, with the destination decoded from
network byte order:

```rust
while let Some(item) = ring.next() {
    let ev: ConnEvent = unsafe { core::ptr::read_unaligned(item.as_ptr() as *const _) };
    let ms = ev.lat_ns as f64 / 1_000_000.0;
    let dest = format!("{}:{}", Ipv4Addr::from(u32::from_be(ev.daddr)), u16::from_be(ev.dport));
    println!("{:<8} {:<16} {:<22} {:.3}", ev.pid, cstr(&ev.comm), dest, ms);
    hist.record(ms, &[KeyValue::new("dport", u16::from_be(ev.dport).to_string())]);
}
```

A `f64_histogram` (not the counter we've used so far) is the right
instrument because latency is a *distribution* — the SDK buckets it, and
Grafana can draw a heatmap or compute p99 without us precomputing
anything. `daddr`/`dport` come off the wire big-endian, so `u32::from_be`
/ `u16::from_be` put them in host order for display.

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

**In Grafana** (`127.0.0.1:3000` → Explore), filter to the `ebpf-tcpconnlat` service and graph `sum by (program) (rate(ebpf_events_total[1m]))` — completed TCP connections as a live rate, the same events your terminal lists, now plotted over time.

## Cross-check

```bash
[vm]$ sudo tcpconnlat-bpfcc
[vm]$ sudo pahole -C sock_common /sys/kernel/btf/vmlinux | grep -E 'skc_daddr|skc_dport'
```

## What you learned

- Part 4 needs **two VMs**; the peer is `provision-vm.sh ebpf-peer`.
- eBPF attaches all along the network path (Figure 27.1); this chapter
  is in the **TCP stack** via kprobes.
- Correlate the two probes of one connection by the **`struct sock *`**
  pointer — the kernel-side entry/exit key.
- Reading kernel struct fields by **offset** works but is fragile;
  **CO-RE** (Part 9) is the portable fix.

Next: **`tcpstates`** traces the whole TCP state machine — with a single
tracepoint that needs none of this offset chasing.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Highest-risk: the `sock_common` offsets (`skc_daddr`@0, `skc_dport`@12 —
CO-RE removes the guesswork); the first-`tcp_rcv_state_process`≈SYN-ACK
assumption; `KProbe::attach` to these symbols in aya 0.13.x; IPv4 only.
The first build and run are the test.*
