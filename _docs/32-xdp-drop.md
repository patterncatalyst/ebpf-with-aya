---
title: "XDP: the earliest hook"
order: 32
part: Networking
description: "Drop and count packets at XDP — the eXpress Data Path, in the driver before an sk_buff is allocated. The fastest eBPF hook, raw packet pointers, and the bounds-checking discipline the verifier demands."
duration: 40 minutes
---

Chapter 31 put your code on the packet path at the qdisc, working on a
fully-formed `sk_buff`. **XDP** — the eXpress Data Path — goes lower and
earlier: it runs in the network driver, on the raw received frame,
*before the kernel allocates an `sk_buff` at all*. That makes it the
fastest place to touch a packet and the canonical home of high-rate
filtering and load balancing — and it changes how you read the packet,
because there's no metadata yet, just bytes between two pointers.

The code is in `examples/32-xdp-drop/`. `./demo.sh` there builds, deploys,
and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="xdp-path"
   alt="The RX path drawn vertically. At the bottom, the NIC driver's RX ring. Immediately above it sits the XDP hook (#[xdp], raw frame, no sk_buff) — the earliest hook. From XDP, XDP_PASS continues up to where the sk_buff is allocated, then to the tc/clsact hook from Chapter 31, then to IP and sockets (Chapters 27-30) at the top. Branching off the XDP hook: XDP_DROP discards the packet immediately, and XDP_TX / XDP_REDIRECT bounce or forward it. The XDP hook runs in the driver, before sk_buff, making it the earliest and fastest hook; tc and the socket hooks all sit higher."
   caption="Figure 32.1 — XDP runs first, in the driver, before any sk_buff exists" %}

## Earliest and fastest — and rawest

Look at the figure top-to-bottom and the whole book's network surface
lines up: sockets and IP at the top (Chapters 27–30), tc/clsact below them
(Chapter 31), and **XDP at the very bottom**, right above the driver. A
packet hits XDP before the kernel has spent anything building an
`sk_buff`, which is exactly why XDP can drop millions of packets per
second — the dropped ones cost almost nothing.

XDP returns a verdict, like tc, but the set is XDP's own:

- `XDP_PASS` — continue up the stack (an `sk_buff` gets built).
- `XDP_DROP` — discard now, in the driver. The cheapest drop in Linux.
- `XDP_TX` — bounce the (possibly rewritten) packet back out the same NIC.
- `XDP_REDIRECT` — send it to another NIC or a userspace socket (AF_XDP).
- `XDP_ABORTED` — error path; behaves like drop and fires a tracepoint.

The cost of being first is that there is **no `sk_buff` and no helper that
copies for you**. You get two pointers — `data` and `data_end` — and
everything between them is the raw frame. Before you read any byte, you
must prove to the verifier that the byte is inside that window.

## How the code works

### The bounds-checked read

Every XDP program has some version of this helper, and it is the heart of
the chapter:

```rust
#[inline(always)]
unsafe fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    if start + offset + mem::size_of::<T>() > end {
        return Err(());                 // would read past the frame — refuse
    }
    Ok((start + offset) as *const T)
}
```

`ctx.data()` and `ctx.data_end()` are the packet window. The check
`start + offset + size_of::<T>() > end` is not optional politeness — it is
the exact condition the **verifier** insists on. The verifier traces the
comparison and only then lets you dereference the pointer; omit it and the
program is rejected at load time, not at runtime. This is the discipline
XDP trades for its speed: you do the bounds proof by hand, every access.

Contrast Chapter 31: `TcContext::load` copied a `T` out of the `sk_buff`
and did the bounds check internally. Here there is nothing to copy out of —
you point directly into driver memory, so you carry the proof yourself.

### The program

```rust
#[xdp]
pub fn xdp_filter(ctx: XdpContext) -> u32 {
    try_filter(&ctx).unwrap_or(xdp_action::XDP_PASS)   // on any miss, pass
}

fn try_filter(ctx: &XdpContext) -> Result<u32, ()> {
    let eth: *const EthHdr = unsafe { ptr_at(ctx, 0)? };
    if unsafe { (*eth).ether_type } != EtherType::Ipv4 {
        return Ok(xdp_action::XDP_PASS);
    }
    let ip: *const Ipv4Hdr = unsafe { ptr_at(ctx, EthHdr::LEN)? };
    let proto = unsafe { (*ip).proto };

    bump(&PKTS, proto as u32, 1);
    if proto == IpProto::Icmp {
        bump(&DROPS, proto as u32, 1);
        return Ok(xdp_action::XDP_DROP);    // ICMP dies here, in the driver
    }
    Ok(xdp_action::XDP_PASS)
}
```

- Each header is obtained through `ptr_at`, so each access is proven in
  bounds before the dereference. We read the Ethernet `ether_type`; if it
  isn't IPv4 we `XDP_PASS` (leave ARP/IPv6/… alone). Then we read the
  IPv4 header and its protocol byte.
- We count every IPv4 packet by protocol in a `HashMap` (same in-kernel
  aggregation as Chapter 31 — per-packet ring buffers are out of the
  question at XDP rates), and we **drop ICMP** with `XDP_DROP`, bumping a
  drop counter. Everything else passes.
- Dropping ICMP makes the verdict vividly visible: a `ping` to the target
  gets no replies while the program is attached, because the echo requests
  are discarded in the driver before the stack ever sees them — then
  resume the instant you detach.

The maps and `bump` are identical in spirit to the previous chapter:

```rust
#[map] static PKTS:  HashMap<u32, u64> = HashMap::with_max_entries(16, 0);
#[map] static DROPS: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);
```

### Attaching XDP (and the VM caveat)

```rust
let iface = std::env::var("IFACE").unwrap_or_else(|_| "eth0".into());
let prog: &mut Xdp = ebpf.program_mut("xdp_filter").unwrap().try_into()?;
prog.load()?;
prog.attach(&iface, XdpFlags::default())
    .or_else(|_| prog.attach(&iface, XdpFlags::SKB_MODE))?;
```

`XdpFlags::default()` asks for **native** (driver) XDP — the fast path,
where the program runs inside the driver's receive routine. Not every
driver supports it. The lab runs on `virtio-net`, which *does* support
native XDP on recent kernels, but to keep the example robust we fall back
to **`SKB_MODE`** (generic XDP): the kernel runs the program a little later,
after a minimal `sk_buff` exists. Generic mode is slower and somewhat
defeats XDP's point, but it works everywhere and is fine for learning and
for functional tests. On bare metal with a supported NIC, native mode is
what you'd run.

Draining is the timer-based delta read from Chapter 31, producing
`ebpf_xdp_packets_total{proto}` and `ebpf_xdp_dropped_total{proto}`.

## Build, deploy, observe

```bash
cd examples/32-xdp-drop && ./demo.sh
```

The demo attaches the filter to the target's interface, then pings the
target from the peer. Before attach, replies come back; with the program
attached, the pings go unanswered (ICMP dropped at XDP) while a parallel
TCP check keeps working — and `ebpf_xdp_packets_total` climbs while
`ebpf_xdp_dropped_total{proto="icmp"}` tracks the dropped echo requests.
Stop the program and ping recovers immediately.

## Cross-check

```bash
[target]$ ip link show <iface>        # shows "xdp" / "xdpgeneric" when attached
[target]$ bpftool net show            # lists the XDP prog bound to the iface
[target]$ ping -c3 <target-ip>        # from the peer: times out while attached
```

`ip link show` reporting `xdp` (native) or `xdpgeneric` (SKB mode) next to
the interface confirms the attach and tells you which mode you got;
`bpftool net show` names the program. The ping going from replies to
timeouts and back is the verdict, demonstrated end to end.

## What you learned

- **XDP** is the earliest, fastest hook — in the driver, before the
  `sk_buff` — with verdicts `XDP_PASS` / `XDP_DROP` / `XDP_TX` /
  `XDP_REDIRECT` / `XDP_ABORTED`.
- The **`ptr_at` bounds-check idiom**: with raw `data`/`data_end`
  pointers you prove every access is in-window before dereferencing, or
  the verifier rejects the program — the trade-off for running rawest.
- The `#[xdp]` program type, `XdpContext`, and attaching with
  `XdpFlags::default()` (native) falling back to `SKB_MODE` (generic),
  including why a VM may need the fallback.

This closes the **Networking** part — from timing connects to dropping
packets in the driver. Next we move to **Part 5, Security & LSM**, where
eBPF programs answer a different question: not "what happened?" but
"should this be allowed?"

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that `virtio-net` accepts native
`XdpFlags::default()` (and otherwise that the `SKB_MODE` fallback
attaches), the `XdpContext::data`/`data_end` signatures and that the
`ptr_at` bounds check satisfies the verifier, the `network-types` field
names, and that `ping` to the target stops while the program is attached
and resumes after detach.*
