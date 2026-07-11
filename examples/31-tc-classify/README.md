# 31 · tc-BPF: acting on packets

A `tc` (clsact) **egress classifier** that counts packets and bytes per L4
protocol and **drops** traffic to a demo port — the first program in this
book that returns a *verdict* and changes what the kernel does, rather than
only observing.

## What it does

- Attaches `#[classifier] tc_classify` to the target's primary interface on
  **egress** (Aya adds the `clsact` qdisc first).
- For each outbound packet: bumps per-protocol `PKTS` and `BYTES` counters
  in kernel-side hash maps.
- If the L4 destination port equals `BLOCK_PORT` (9999), bumps `DROPS` and
  returns `TC_ACT_SHOT` — the packet never reaches the wire. Everything
  else returns `TC_ACT_OK`.
- User space reads the maps every 2s and exports deltas as
  `ebpf_tc_packets_total{proto}`, `ebpf_tc_bytes_total{proto}`, and
  `ebpf_tc_dropped_total{proto}`.

## Run it

```bash
./demo.sh           # build + deploy to $VM (default ebpf-target) + drive traffic
./demo.sh build     # just build on the host
IFACE=enp1s0 ...    # override the interface (the demo auto-detects it otherwise)
```

Needs the two-VM lab (target + `ebpf-peer`) and the Chapter 3 stack, as in
Chapter 27. Drive traffic to `:9100` (passes, counted) and `:9999` (dropped);
the dropped connections time out on the target because their packets are
discarded on egress.

## Verify on the target

```bash
tc qdisc show dev <iface>            # clsact present
tc filter show dev <iface> egress    # the BPF classifier attached
tc -s qdisc show dev <iface>         # kernel-side stats/drops
```

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM (Fedora 44, kernel 7.1.3-200.fc44): the classifier builds, loads,
and attaches to the `clsact` egress hook, counts packets and bytes per
protocol, and drops traffic to the demo port (`TC_ACT_SHOT`) as described.
Attach targets and struct offsets (`network-types` field names,
`ctx.load`/`ctx.len` signatures) can be kernel-version-specific.
