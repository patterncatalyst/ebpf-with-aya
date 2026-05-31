---
title: "XDP load balancer: rewriting packets"
order: 34
part: Networking
description: "Make XDP forward, not just observe: a UDP load balancer that rewrites the destination port of packets aimed at a virtual port, round-robins them across backends through a map, and passes them on — the packet-mutation and backend-selection core of an XDP load balancer."
duration: 45 minutes
---

So far XDP has dropped (Chapter 32) and observed (Chapter 33). Its third
power is to **rewrite and forward** — change bytes in the packet and send it
on its way. That is the heart of an XDP load balancer like Katran: a virtual
address out front, a set of backends behind it, and an XDP program that
rewrites each incoming packet to a chosen backend at line rate. This chapter
builds a deliberately small one — a **UDP port load balancer** — so the two
genuinely new skills stand out: **mutating the packet** (with the verifier
still watching every access) and **selecting a backend** from a map.

The code is in `examples/34-xdp-lb/`. `./demo.sh` there builds, deploys, and
runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="xdp-lb"
   alt="A client on the peer sends UDP datagrams to a virtual port (VIP:8080) on the target. The XDP load balancer rewrites the destination port round-robin using BACKENDS[idx % N], sending roughly one third each to backend :9001, :9002, and :9003. XDP_PASS delivers locally; XDP_REDIRECT would forward to backends on other hosts."
   caption="Figure 34.1 — One virtual port fanned out across backends by rewriting the UDP dest port" %}

## Why UDP, and why a port

A real load balancer has to solve the *return path*: once you send a
client's packet to a backend, the backend's reply must look like it came
from the virtual address, or the client rejects it. Production XDP balancers
handle this with Direct Server Return or a connection-tracking map — real,
and a chapter of their own later. To keep *this* chapter about packet
mutation and backend selection, we use **UDP** (connectionless, so
per-packet balancing is correct) and rewrite only the **destination port**,
fanning a single virtual port across several local backend listeners. The
production shape — rewriting addresses and using `XDP_REDIRECT` to backends
on other hosts — is called out at the end.

## How the code works

### Backends and the round-robin index

```rust
#[map] static BACKENDS: Array<u16>      = Array::with_max_entries(8, 0);  // backend ports
#[map] static NBACK:    Array<u32>      = Array::with_max_entries(1, 0);  // how many are set
#[map] static IDX:      Array<u32>      = Array::with_max_entries(1, 0);  // round-robin cursor
#[map] static HITS:     HashMap<u16,u64>= HashMap::with_max_entries(8, 0);// per-backend counter
```

The loader fills `BACKENDS` with the backend ports and `NBACK` with the
count, so the set is configurable without recompiling. `IDX` is a single
cursor the program advances per packet; `HITS` counts dispatches per backend
for observability.

### Mutating the packet

```rust
const VIP_PORT: u16 = 8080;

fn try_lb(ctx: &XdpContext) -> Result<u32, ()> {
    let eth: *const EthHdr = unsafe { ptr_at(ctx, 0)? };
    if unsafe { (*eth).ether_type } != EtherType::Ipv4 { return Ok(xdp_action::XDP_PASS); }
    let ip: *const Ipv4Hdr = unsafe { ptr_at(ctx, EthHdr::LEN)? };
    if unsafe { (*ip).proto } != IpProto::Udp { return Ok(xdp_action::XDP_PASS); }

    let udp_off = EthHdr::LEN + Ipv4Hdr::LEN;
    let udp: *mut UdpHdr = unsafe { ptr_at_mut(ctx, udp_off)? };       // mutable view
    if unsafe { u16::from_be((*udp).dest) } != VIP_PORT { return Ok(xdp_action::XDP_PASS); }

    // pick a backend: BACKENDS[idx % n]
    let n = *unsafe { NBACK.get(0) }.ok_or(())?;
    if n == 0 { return Ok(xdp_action::XDP_PASS); }
    let cur = *unsafe { IDX.get(0) }.ok_or(())?;
    let port = *unsafe { BACKENDS.get(cur % n) }.ok_or(())?;
    if let Some(slot) = IDX.get_ptr_mut(0) { unsafe { *slot = cur.wrapping_add(1); } }

    // rewrite: dest port → backend, and zero the (optional) UDP checksum
    unsafe {
        (*udp).dest = port.to_be();
        (*udp).check = 0;          // IPv4 UDP checksum is optional; 0 = "not computed"
    }
    bump(&HITS, port, 1);
    Ok(xdp_action::XDP_PASS)       // local stack now delivers to the chosen backend
}
```

The new moves:

- **`ptr_at_mut`** is `ptr_at` returning a `*mut T`: the same bounds proof,
  then a writable pointer. The verifier requires the in-window check before a
  **write** just as for a read — you cannot scribble past `data_end`.
- **Backend selection** reads the count and cursor from single-entry
  `Array` maps, indexes `BACKENDS[cur % n]`, and advances the cursor with
  `IDX.get_ptr_mut(0)` (a writable pointer into the map value). Round-robin
  in three map reads and one write.
- **The rewrite** sets `(*udp).dest` to the backend port (network order) and
  zeros `(*udp).check`. Zeroing is legitimate here: in IPv4 the UDP checksum
  is optional, and `0` means "not computed," so we sidestep checksum math for
  the demo. **For TCP, or to keep the UDP checksum valid, you must do an
  incremental update** (RFC 1624: subtract the old port, add the new) rather
  than recompute from scratch — the standard XDP technique, noted here and
  used in the production version.
- We `XDP_PASS`, so the local network stack delivers the now-rewritten
  datagram to whichever backend is listening on the chosen port.

### User side: program the backends, watch the spread

```rust
let backends: Vec<u16> = vec![9001, 9002, 9003];
{
    let mut b: Array<_, u16> = Array::try_from(ebpf.map_mut("BACKENDS").unwrap())?;
    for (i, p) in backends.iter().enumerate() { b.set(i as u32, *p, 0)?; }
    let mut nb: Array<_, u32> = Array::try_from(ebpf.map_mut("NBACK").unwrap())?;
    nb.set(0, backends.len() as u32, 0)?;
}
// attach XDP (native → SKB fallback), then drain HITS into ebpf_xdp_lb_dispatch_total{backend}
```

The loader writes the backend ports and count into the maps *before*
attaching, then drains `HITS` on a timer into
`ebpf_xdp_lb_dispatch_total{backend}` and prints a per-backend table. Change
the `backends` vector and rerun — no recompile, because the set lives in maps.

## Build, deploy, observe

```bash
cd examples/34-xdp-lb && ./demo.sh
```

The demo starts three UDP listeners on the target (ports 9001–9003), attaches
the balancer, then fires a stream of UDP datagrams from the peer at the
target's `VIP:8080`. Each datagram is rewritten to one of the three backends
in turn, so all three listeners receive roughly a third — visible both in the
listeners' output and as three roughly-equal `ebpf_xdp_lb_dispatch_total`
series in Grafana.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `sum by (backend) (rate(ebpf_xdp_lb_dispatch_total[1m]))` — dispatch rate per backend — the load balance made visible.

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`):

```bash
[vm]$ ip -br link                 # find the NIC (e.g. enp1s0)
[vm]$ sudo bpftool net show       # confirm the XDP program is attached to it
[vm]$ sudo ss -ulnp | grep -E '9001|9002|9003'   # the three backend listeners are up
```

While traffic flows, watch a backend actually receive rewritten datagrams —
e.g. point `tcpdump` at one backend port and confirm packets the client sent
to `8080` arrive on `9001`:

```bash
[vm]$ sudo tcpdump -ni enp1s0 udp port 9001    # client sent to :8080, arriving on :9001
```

Seeing datagrams the client addressed to `8080` show up on `9001`/`9002`/`9003`
is the rewrite working; the roughly even split across the three is the
round-robin.

## What you learned

- XDP can **rewrite and forward**, not just drop or observe — the third
  verdict family, and the basis of XDP load balancers.
- **`ptr_at_mut`**: writing into the packet needs the same verifier bounds
  proof as reading, and the UDP checksum can be zeroed in IPv4 (but TCP /
  valid UDP need an **incremental checksum update**).
- **Map-driven backend selection**: ports and count in `Array` maps, a
  round-robin cursor advanced with `get_ptr_mut`, per-backend counters —
  reconfigurable without recompiling.

That closes the **Networking** part. We built up from timing a connection to
dropping, capturing, and load-balancing packets in the driver. Next is
**Part 5, Security & LSM**, where eBPF stops reporting on the kernel and
starts making allow/deny decisions for it.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: `ptr_at_mut` writes passing the verifier,
that zeroing the IPv4 UDP checksum is accepted end-to-end (datagrams arrive
on the rewritten port), the `Array` map read/write API
(`get`/`get_ptr_mut`/`set`) from both sides, `XdpFlags` native-vs-`SKB_MODE`
on `virtio-net`, and that the three backends receive a roughly even split.*
