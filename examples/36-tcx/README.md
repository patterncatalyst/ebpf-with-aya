# 36 · tcx: the modern tc attach

The Chapter 31 classifier, reattached with **tcx** (kernel 6.6+): no clsact
qdisc, a `bpf_link` that auto-detaches when dropped, and a kernel-managed
order that lets several programs share a hook.

## What it does

- `tcx-ebpf` is a minimal ingress classifier — count packets per protocol
  in `PKTS`, return `TC_ACT_OK`. Identical in shape to Chapter 31.
- `tcx` (loader) loads it and attaches with `SchedClassifier::attach(&iface,
  TcAttachType::Ingress)` — **no `qdisc_add_clsact`**. The returned link
  owns the attachment; dropping it (on Ctrl-C) detaches.
- Exports `ebpf_tcx_packets_total{proto}`.

## Run it

```bash
./demo.sh          # build + deploy to $VM + drive traffic from the peer
./demo.sh build    # just build on the host
IFACE=enp1s0 ...   # override the interface (auto-detected otherwise)
```

Needs a target on **kernel ≥ 6.6** (tcx). The counts rise like Chapter 31,
but the attach mechanism is entirely different.

## Verify on the target — the contrast with Chapter 31

```bash
ip -br link                          # find the NIC (e.g. enp1s0)
sudo bpftool net show                # your program is listed under tcx/ingress
sudo tc filter show dev enp1s0 ingress   # EMPTY — tcx is not a tc filter
sudo tc qdisc show dev enp1s0        # no clsact qdisc was created
```

`bpftool net show` listing `tcx/ingress` while `tc filter show` is empty and
no clsact qdisc exists is the proof that tcx is a distinct, kernel-managed
attach point.

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches via
`SchedClassifier::attach` with `TcAttachType::Ingress`, and runs as
described — `bpftool net show` lists the program under `tcx/ingress` while
`tc filter show` is empty and no clsact qdisc is created. The kernel ≥ 6.6
requirement for tcx is satisfied by 7.1.3. Attach targets and struct
offsets can be kernel-version-specific.
