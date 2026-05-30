---
title: "tc-BPF: acting on packets"
order: 31
part: Networking
description: "Move from observing traffic to acting on it: attach a classifier to the clsact qdisc, count egress packets per protocol in-kernel, and drop traffic to a port with a TC_ACT_SHOT verdict."
duration: 40 minutes
---

Every networking program so far has *watched*: Chapter 27 timed connects,
28 tracked state changes, 29 read request lines, 30 noted established
sockets. None of them changed what the kernel did with a packet. This
chapter crosses that line. A `tc` classifier sits on the packet path and
returns a **verdict** — pass it, drop it, or send it somewhere else — so
for the first time your eBPF program is part of the data path, not a tap
beside it.

The code is in `examples/31-tc-classify/`. `./demo.sh` there builds,
deploys, and runs it; its `README.md` covers what it does and how to
drive it.

{% include excalidraw.html
   file="tc-clsact"
   alt="The clsact qdisc on an interface carries a tc ingress hook and a tc egress hook. Packets received from the wire pass through the ingress classifier on the way up to the network stack; packets sent by the stack pass through the egress classifier on the way down to the wire. Each #[classifier] program operates on the sk_buff and returns a verdict: TC_ACT_OK to pass, TC_ACT_SHOT to drop, or TC_ACT_REDIRECT to send to another interface."
   caption="Figure 31.1 — tc runs on the sk_buff, in both directions, and returns a verdict" %}

## The shift: from observing to acting

`tc` (traffic control) BPF programs attach to the **clsact qdisc**, a
queueing discipline that exists only to host eBPF classifiers. It gives
you two hooks per interface — **ingress** and **egress** — so unlike XDP
(next chapter, ingress-only) you can act on outbound traffic too.

A classifier runs on the **`sk_buff`**: the packet *plus* the metadata the
kernel has already attached (the interface, the protocol, checksums). That
makes it a gentle first step into the data path — you have structured
access to the packet, not just raw bytes — and the program's **return
value is a verdict** the kernel obeys:

- `TC_ACT_OK` — let the packet continue.
- `TC_ACT_SHOT` — drop it.
- `TC_ACT_REDIRECT` — send it out a different interface (the basis of the
  load balancers and bridges built on tc).

Our program counts every egress packet by protocol, and — to show the
verdict is real — drops anything headed for a demo port.

## How the code works

### Maps: aggregate in the kernel, not per-packet

The networking observers used a `RingBuf` to ship one record per event.
On a data path that would be a catastrophe: egress fires for *every packet*,
and a ring would fill and drop almost immediately while drowning user space
in wakeups. So tc and XDP programs follow the **Performance-part** lesson
instead — aggregate in the kernel, read totals on a timer:

```rust
#[map] static PKTS:  HashMap<u32, u64> = HashMap::with_max_entries(16, 0);
#[map] static BYTES: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);
#[map] static DROPS: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);
```

Three small hash maps keyed by L4 protocol number (TCP = 6, UDP = 17,
ICMP = 1). Each packet bumps a counter; user space reads the totals every
couple of seconds. The kernel side stays O(1) per packet with no buffering.

### The classifier

```rust
#[classifier]
pub fn tc_classify(ctx: TcContext) -> i32 {
    try_classify(&ctx).unwrap_or(TC_ACT_OK)   // on any parse miss, never break traffic
}

fn try_classify(ctx: &TcContext) -> Result<i32, ()> {
    let eth: EthHdr = ctx.load(0).map_err(|_| ())?;
    if eth.ether_type != EtherType::Ipv4 { return Ok(TC_ACT_OK); }
    let ip: Ipv4Hdr = ctx.load(EthHdr::LEN).map_err(|_| ())?;
    let proto = ip.proto as u32;

    bump(&PKTS, proto, 1);
    bump(&BYTES, proto, ctx.len() as u64);

    let l4 = EthHdr::LEN + Ipv4Hdr::LEN;
    let dport = match ip.proto {
        IpProto::Tcp => u16::from_be(ctx.load::<TcpHdr>(l4).map_err(|_| ())?.dest),
        IpProto::Udp => u16::from_be(ctx.load::<UdpHdr>(l4).map_err(|_| ())?.dest),
        _ => 0,
    };
    if dport == BLOCK_PORT {
        bump(&DROPS, proto, 1);
        return Ok(TC_ACT_SHOT);          // the verdict: this packet does not leave
    }
    Ok(TC_ACT_OK)
}
```

Walking it the way you would write it:

- **`ctx.load::<T>(offset)`** copies `T` out of the `sk_buff` at a byte
  offset, bounds-checked by the kernel — the tc analogue of the
  socket-filter `load` from Chapter 29. We pull the Ethernet header at 0,
  bail to `TC_ACT_OK` for anything that isn't IPv4 (don't disturb ARP,
  IPv6, …), then pull the IPv4 header at `EthHdr::LEN`. The `EthHdr` /
  `Ipv4Hdr` / `TcpHdr` types come from the `network-types` crate, so the
  offsets and field names are named rather than magic numbers.
- **`ctx.len()`** is the `sk_buff` length — the per-packet byte count we
  add to `BYTES`, giving a throughput signal for free.
- The **verdict** is the whole point. We read the L4 destination port
  (TCP or UDP), and if it matches `BLOCK_PORT` we return `TC_ACT_SHOT` —
  the kernel discards the packet and it never reaches the wire. Every
  other packet returns `TC_ACT_OK` and continues untouched.
- **`unwrap_or(TC_ACT_OK)`** at the top is a safety policy: if a packet is
  malformed or too short to parse, we pass it rather than risk dropping
  legitimate traffic. A firewall would invert that; an observer passes.

`bump` is the now-familiar read-modify-write on a per-key counter:

```rust
#[inline(always)]
fn bump(m: &HashMap<u32, u64>, key: u32, by: u64) {
    let new = unsafe { m.get(&key).copied().unwrap_or(0) } + by;
    let _ = m.insert(&key, &new, 0);
}
```

### The user side: make the qdisc, attach, drain

```rust
let iface = std::env::var("IFACE").unwrap_or_else(|_| "eth0".into());

// clsact doesn't exist by default — add it (ignore "already exists").
let _ = tc::qdisc_add_clsact(&iface);

let prog: &mut SchedClassifier = ebpf.program_mut("tc_classify").unwrap().try_into()?;
prog.load()?;
prog.attach(&iface, TcAttachType::Egress)?;
```

Two things are new versus the socket-layer chapters. First, tc needs a
host qdisc to attach to, so we ask Aya to add **clsact** to the interface
(idempotent — if it's already there the error is harmless). Second,
`attach` takes a **`TcAttachType`** — `Egress` here; `Ingress` would watch
inbound. Everything else is the familiar cast → `load` (verifier) →
`attach` (target) sequence.

Reading aggregation maps from user space is the Performance-part pattern
(Chapter 22): take the map, then on a timer iterate its entries and report
deltas as OTLP counters:

```rust
let pkts: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("PKTS").unwrap())?;
// … BYTES, DROPS likewise, plus a `last` snapshot per map for deltas …
loop {
    tokio::time::sleep(Duration::from_secs(2)).await;
    for res in pkts.iter() {
        let (proto, total) = res?;
        let delta = total - last.get(&proto).copied().unwrap_or(0);
        if delta > 0 {
            packets_total.add(delta, &[KeyValue::new("proto", proto_name(proto))]);
            last.insert(proto, total);
        }
    }
}
```

We report **deltas** because OTLP counters are cumulative and so are the
kernel totals; subtracting the last snapshot turns "total so far" into
"how many since I last looked." The result is `ebpf_tc_packets_total`,
`ebpf_tc_bytes_total`, and `ebpf_tc_dropped_total`, each labelled by
protocol.

## Build, deploy, observe

```bash
cd examples/31-tc-classify && ./demo.sh
```

The demo resolves the target's primary interface, attaches the egress
classifier there, then drives traffic from the target to the peer: ordinary
requests to a normal port (passed and counted) and a burst aimed at the
demo port (dropped). In Grafana you'll see `ebpf_tc_packets_total` rising
across protocols and `ebpf_tc_dropped_total` ticking up for the blocked
port — and on the target, the connections to that port simply time out,
because their packets never leave the box.

## Cross-check

```bash
[target]$ tc qdisc show dev <iface>          # the clsact qdisc is present
[target]$ tc filter show dev <iface> egress  # the BPF classifier is attached
[target]$ tc -s qdisc show dev <iface>       # counters/drops at the qdisc level
```

`tc filter show … egress` listing your program confirms the attach; the
`-s` stats give the kernel's own view of what the qdisc dropped, next to
your `ebpf_tc_dropped_total`.

## What you learned

- `tc`/clsact programs run on the **`sk_buff`** in **both directions** and
  return a **verdict** (`TC_ACT_OK` / `TC_ACT_SHOT` / `TC_ACT_REDIRECT`) —
  your code is now in the data path, not beside it.
- The `#[classifier]` program type, `TcContext::load`/`len`, and attaching
  with `tc::qdisc_add_clsact` + `SchedClassifier` + `TcAttachType`.
- Why data-path programs **aggregate in-kernel** and report on a timer
  instead of emitting per-packet, and how to read those maps as deltas.

Next, Chapter 32 drops to the earliest hook of all — **XDP**, in the
driver, before the `sk_buff` even exists.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: the Aya `tc` API surface
(`qdisc_add_clsact`, `SchedClassifier`, `TcAttachType::Egress`), the
`network-types` header/field names and the `ctx.load`/`ctx.len`
signatures, that `TC_ACT_SHOT` actually drops on egress (connections to
the demo port time out), and the `HashMap::iter()` read path from user
space.*
