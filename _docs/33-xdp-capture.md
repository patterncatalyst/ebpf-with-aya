---
title: "XDP capture: a tcpdump in eBPF"
order: 33
part: Networking
description: "Use XDP to observe rather than drop: parse and filter packets in the kernel, and ship only the interesting ones (TCP SYN/FIN/RST) to user space through a ring buffer for a live tcpdump-style view of connection activity."
duration: 40 minutes
---

Chapter 32 used XDP to *drop*. The same hook is just as good at *watching* —
and watching at XDP is how high-rate packet capture is built, because you
can decide in the kernel which packets are worth user space's attention and
discard the rest for free. This chapter builds a miniature `tcpdump`: an XDP
program that parses each frame, keeps only TCP control packets
(SYN / FIN / RST — the start and end of connections), and ships a one-line
record for each to user space. Every other packet is passed untouched.

The code is in `examples/33-xdp-capture/`. `./demo.sh` there builds,
deploys, and runs it; its `README.md` covers what it does and how to drive
it.

{% include excalidraw.html
   file="xdp-capture"
   alt="A frame arrives at the NIC RX path and enters the XDP capture program, which parses and filters it in the kernel. Matching packets (TCP SYN/FIN/RST) have a small record copied into a RingBuf that user space drains into tcpdump-style lines. Every packet — matching or not — continues up the network stack via XDP_PASS. The point: capture is read-only and filters in-kernel, shipping only matching records, so you never send every packet to user space."
   caption="Figure 33.1 — Filter in the kernel, ship only the matches" %}

## The discipline: filter first, copy little

Chapters 31 and 32 hammered one rule: on a data path you don't ship a
record per packet, you aggregate. Capture looks like the exception — its
whole job is to emit records — but the rule still holds, just shifted: you
**filter in the kernel** so that only a tiny fraction of packets ever
become a record, and you **copy only what you need** (a few header fields),
never the whole frame. A SYN-only filter on a busy host emits a handful of
records per second, not millions. That's what makes a `RingBuf` the right
tool here where it was the wrong one for a packet counter.

So this program returns `XDP_PASS` for everything — it changes nothing — and
its only side effect is dropping a small `FlowRecord` into a ring when a
packet matches.

## How the code works

### The record and the maps

```rust
// in -common, shared with user space
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlowRecord {
    pub saddr: u32, pub daddr: u32,   // network byte order
    pub sport: u16, pub dport: u16,
    pub flags: u8,                    // the TCP flags byte
    pub len:   u16,                   // total IP length
}
```

```rust
// in -ebpf
#[map] static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);
#[map] static SEEN:   HashMap<u32, u64> = HashMap::with_max_entries(4, 0); // per-proto totals
```

One ring for the matched records, and a tiny `SEEN` map so we can also
report *how many packets went by* — the denominator that makes the capture
count meaningful (captured 12 of 40,000 seen).

### Parsing and filtering at XDP

```rust
const TCP_FIN: u8 = 0x01;
const TCP_SYN: u8 = 0x02;
const TCP_RST: u8 = 0x04;

fn try_capture(ctx: &XdpContext) -> Result<u32, ()> {
    let eth: *const EthHdr = unsafe { ptr_at(ctx, 0)? };
    if unsafe { (*eth).ether_type } != EtherType::Ipv4 { return Ok(xdp_action::XDP_PASS); }
    let ip: *const Ipv4Hdr = unsafe { ptr_at(ctx, EthHdr::LEN)? };
    let proto = unsafe { (*ip).proto };
    bump(&SEEN, proto as u32, 1);

    if proto != IpProto::Tcp { return Ok(xdp_action::XDP_PASS); }
    let tcp_off = EthHdr::LEN + Ipv4Hdr::LEN;
    let tcp: *const TcpHdr = unsafe { ptr_at(ctx, tcp_off)? };
    let flags: u8 = unsafe { *ptr_at::<u8>(ctx, tcp_off + 13)? };  // flags byte

    if flags & (TCP_SYN | TCP_FIN | TCP_RST) != 0 {
        if let Some(mut slot) = EVENTS.reserve::<FlowRecord>(0) {
            let rec = FlowRecord {
                saddr: unsafe { (*ip).src_addr }, daddr: unsafe { (*ip).dst_addr },
                sport: unsafe { u16::from_be((*tcp).source) },
                dport: unsafe { u16::from_be((*tcp).dest) },
                flags,
                len: unsafe { u16::from_be((*ip).tot_len) },
            };
            unsafe { *slot.as_mut_ptr() = rec; }
            slot.submit(0);
        }
    }
    Ok(xdp_action::XDP_PASS)
}
```

Reading it the way you'd write it:

- Every header is fetched through the **`ptr_at` bounds check** from
  Chapter 32 — capture is still raw-pointer XDP, so each access is proven
  in-window before the dereference.
- We count *every* IPv4 packet by protocol in `SEEN` (the denominator),
  then narrow to TCP.
- The **flags byte** lives at offset 13 of the TCP header. Rather than lean
  on a bitfield accessor, we read the byte directly with
  `ptr_at::<u8>(ctx, tcp_off + 13)` — the same offset-reading habit from the
  socket chapters, and unambiguous. `SYN | FIN | RST` is our filter: connection
  setup and teardown, which is what you usually want to *see*.
- Only on a match do we touch the ring: `reserve` a `FlowRecord`, fill it
  (addresses kept in network order; ports and length byte-swapped to host
  order), and `submit`. A non-match costs one comparison and a counter bump.
- The verdict is always `XDP_PASS` — we are a tap, not a filter.

### User side: drain into tcpdump-style lines

```rust
let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;
// … on a short timer …
while let Some(item) = ring.next() {
    let r: FlowRecord = unsafe { core::ptr::read_unaligned(item.as_ptr() as *const _) };
    let names = flag_names(r.flags);   // e.g. "SYN", "FIN ACK", "RST"
    println!("{:<10} {}:{} > {}:{}  len {}",
        names,
        Ipv4Addr::from(u32::from_be(r.saddr)), r.sport,
        Ipv4Addr::from(u32::from_be(r.daddr)), r.dport,
        r.len);
    captured.add(1, &[KeyValue::new("flag", primary_flag(r.flags))]);
}
```

The drain is the same `RingBuf` loop you've used since Chapter 9, with a
`read_unaligned` because the record is packed. The output reads like
`tcpdump`: `SYN  10.0.0.5:54312 > 10.0.0.9:443  len 60`. We also export
`ebpf_xdp_captured_total{flag}` (and a gauge of `SEEN` totals), so Grafana
shows the capture rate against the total packet rate.

## Build, deploy, observe

```bash
cd examples/33-xdp-capture && ./demo.sh
```

The demo attaches the capture program to the target's interface, then opens
and closes connections from the peer (a loop of short `curl`s). You'll see a
`SYN` line as each connection starts and a `FIN`/`RST` as it ends — a live
feed of connection activity — while `ebpf_xdp_captured_total` rises in
Grafana far more slowly than the underlying packet count.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `rate(ebpf_xdp_seen_total[1m])` — packets seen, against `rate(ebpf_xdp_captured_total[1m])` for how many matched the filter.

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`),
find the interface (usually `enp1s0`) and run real `tcpdump` with the same
filter beside your program:

```bash
[vm]$ ip -br link                              # find the NIC (e.g. enp1s0)
[vm]$ sudo tcpdump -ni enp1s0 'tcp[tcpflags] & (tcp-syn|tcp-fin|tcp-rst) != 0'
```

`tcpdump`'s `tcp[tcpflags] & (…) != 0` filter is exactly the SYN/FIN/RST
test your XDP program does — its lines should match yours packet for packet.
That's the most direct "is my capture correct?" check there is.

## What you learned

- XDP is as useful for **observing** as for dropping: parse and **filter in
  the kernel**, copy only the fields you need, and `XDP_PASS` everything so
  you change nothing.
- Reading the **TCP flags byte** by offset, and using a `RingBuf` correctly
  on a data path — legitimate precisely *because* the in-kernel filter keeps
  the record rate low.
- A SYN/FIN/RST feed is a connection-activity view that mirrors
  `tcpdump 'tcp[tcpflags] & … != 0'`.

Next, Chapter 34 makes XDP *forward*: a small load balancer that rewrites
packets and fans them across backends with `XDP_TX`/`XDP_PASS`.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
