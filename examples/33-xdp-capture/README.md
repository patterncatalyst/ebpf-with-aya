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

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab
VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches, and runs as
described — the `RingBuf` inside `#[xdp]` passes the verifier, the
`network-types` field names resolve, and the SYN/FIN/RST filter (flags byte at
TCP offset 13) captures matching packets. Attach targets and struct offsets can
be kernel-version-specific.
