# 34 · XDP load balancer: rewriting packets

A minimal **UDP load balancer** at XDP. Datagrams aimed at a virtual port
(`VIP:8080`) have their destination port rewritten round-robin across a set
of backends, then are passed up so the local stack delivers them — the
packet-mutation and backend-selection core of an XDP load balancer.

## What it does

- The loader fills `BACKENDS` (default `9001,9002,9003`, override with
  `$BACKENDS`) and `NBACK`, then attaches `#[xdp] xdp_lb` (native →
  `SKB_MODE`).
- For each UDP datagram to `VIP_PORT` (8080): picks `BACKENDS[idx % n]`,
  advances the round-robin cursor, rewrites the UDP destination port via a
  mutable packet pointer, zeroes the (optional, IPv4) UDP checksum, bumps a
  per-backend counter, and returns `XDP_PASS`.
- Exports `ebpf_xdp_lb_dispatch_total{backend}`.

> **Scope:** UDP and port-only rewrite keep this about *mutation + selection*.
> A production balancer also handles the return path (DSR or a conntrack map)
> and rewrites addresses with `XDP_REDIRECT` to backends on other hosts, and
> uses an **incremental checksum update** (RFC 1624) instead of zeroing —
> noted in the chapter.

## Run it

```bash
./demo.sh                       # build + deploy + start backends + send UDP from peer
./demo.sh build                 # just build on the host
BACKENDS=9001,9002 ./demo.sh    # change the backend set (no recompile)
```

Needs the two-VM lab. Watch the split on the target with
`tail -f /tmp/backend-90*.log`, or the three roughly-equal
`ebpf_xdp_lb_dispatch_total` series in Grafana.

## Verify on the target

```bash
ip -br link                                  # find the NIC (e.g. enp1s0)
sudo bpftool net show                        # XDP program attached
sudo ss -ulnp | grep -E '9001|9002|9003'     # the three backend listeners
sudo tcpdump -ni enp1s0 udp port 9001        # client sent to :8080, arriving on :9001
```

## Verification status

**Unverified** — written against Aya 0.13 / aya-ebpf 0.1 / network-types
0.0.7, not yet run on Fedora 44. Confirm: `ptr_at_mut` writes passing the
verifier, that zeroing the IPv4 UDP checksum is accepted end-to-end, the
`Array` `get`/`get_ptr_mut`/`set` API both sides, `XdpFlags` native vs
`SKB_MODE` on `virtio-net`, and a roughly even split across backends.
