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

**Unverified** — needs kernel ≥ 6.6. Confirm the Aya tcx attach API
(whether `SchedClassifier::attach` with `TcAttachType::Ingress` selects tcx
and returns a link, or a distinct call/options are needed), that the link's
lifetime governs detach, that `bpftool net show` reports `tcx/ingress`, and
that no clsact qdisc is created.
