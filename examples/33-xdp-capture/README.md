# 33 · XDP capture: a tcpdump in eBPF

An XDP program that parses each frame, keeps only TCP control packets
(SYN/FIN/RST), and ships a one-line record per match to user space — a
miniature `tcpdump` showing connection setup/teardown. Read-only: every
packet returns `XDP_PASS`.

## What it does

- Attaches `#[xdp] xdp_capture` to the target's interface (native, falling
  back to `SKB_MODE`).
- Counts every IPv4 packet by protocol in `SEEN`; for TCP packets whose
  flags byte has SYN/FIN/RST set, ships a `FlowRecord` (4-tuple + flags +
  length) through a `RingBuf`.
- User space prints `tcpdump`-style lines and exports
  `ebpf_xdp_captured_total{flag}` and `ebpf_xdp_seen_total{proto}`.

## Run it

```bash
./demo.sh          # build + deploy to $VM + open/close connections from the peer
./demo.sh build    # just build on the host
IFACE=enp1s0 ...   # override the interface (auto-detected otherwise)
```

You'll see a `SYN` line as each connection starts and `FIN`/`RST` as it
ends; the capture rate stays low because the filter runs in the kernel.

## Verify on the target

```bash
ip -br link                                                   # find the NIC (e.g. enp1s0)
sudo tcpdump -ni enp1s0 'tcp[tcpflags] & (tcp-syn|tcp-fin|tcp-rst) != 0'
```

The `tcpdump` filter is the same SYN/FIN/RST test — its lines should match
yours.

## Verification status

**Unverified** — written against Aya 0.13 / aya-ebpf 0.1 / network-types
0.0.7, not yet run on Fedora 44. Confirm: `RingBuf` inside `#[xdp]`, the
`network-types` field names (`src_addr`/`dst_addr`/`tot_len`/`source`/`dest`),
the flags byte at TCP offset 13, `ptr_at` passing the verifier, and that the
captured lines match `tcpdump`.
