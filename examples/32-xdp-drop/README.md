# 32 · XDP: the earliest hook

An **XDP** ingress filter that counts IPv4 packets per protocol and **drops
ICMP** — running in the driver, before an `sk_buff` is allocated. The
fastest place in Linux to drop a packet.

## What it does

- Attaches `#[xdp] xdp_filter` to the target's primary interface, trying
  native (driver) XDP first and falling back to generic `SKB_MODE`.
- Parses Ethernet + IPv4 using raw `data`/`data_end` pointers with an
  explicit `ptr_at` bounds check (the verifier requires it).
- Counts every IPv4 packet by protocol in `PKTS`; for ICMP, bumps `DROPS`
  and returns `XDP_DROP`. Everything else returns `XDP_PASS`.
- Exports `ebpf_xdp_packets_total{proto}` and
  `ebpf_xdp_dropped_total{proto}` (deltas read every 2s).

## Run it

```bash
./demo.sh           # build + deploy to $VM (default ebpf-target) + ping from peer
./demo.sh build     # just build on the host
IFACE=enp1s0 ...    # override the interface (auto-detected otherwise)
```

While the program is attached, `ping <target>` from the peer times out
(ICMP dropped in the driver) and recovers the moment you stop it; TCP/SSH
keep working throughout.

## Verify on the target

```bash
ip link show <iface>      # shows "xdp" (native) or "xdpgeneric" (SKB mode)
bpftool net show          # names the XDP program bound to the interface
```

## Verification status

**Unverified** — written against Aya 0.13 / aya-ebpf 0.1 / network-types
0.0.7 but not yet run on Fedora 44. Confirm: that `virtio-net` accepts
native `XdpFlags::default()` (else the `SKB_MODE` fallback attaches), the
`XdpContext::data`/`data_end` signatures and that `ptr_at` satisfies the
verifier, the `network-types` field names, and that `ping` to the target
stops while attached and resumes after detach.
