---
title: "tcx: the modern tc attach"
order: 36
part: Networking
description: "Attach tc programs the modern way: tcx (kernel 6.6+) replaces the clsact-qdisc wiring with bpf_link ownership and a kernel-managed ordering of multiple programs per hook — auto-detach on close, no manual qdisc, and composable with other tools."
duration: 35 minutes
---

Chapter 31 attached a tc classifier the original way: add a `clsact`
qdisc by hand, then hang a BPF filter off it. That works, but it has rough
edges — you manage the qdisc's lifetime yourself, only one filter owns a
priority slot, and nothing coordinates with other tools that want the same
hook. Kernel 6.6 introduced **`tcx`**, a `bpf_link`-based attachment that
fixes all three: no qdisc to manage, several programs can share a hook in a
**kernel-managed order**, and the attachment is owned by a link that
**auto-detaches when dropped**. This chapter reattaches a classifier with
`tcx` and shows what changes.

The code is in `examples/36-tcx/`. `./demo.sh` there builds, deploys, and
runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="tcx-chain"
   alt="An ingress packet enters a tcx chain — a kernel-ordered, bpf_link-based sequence of classifier programs (prog A then prog B) — and the result goes up to the network stack. tcx attaches via bpf_link with several programs in a kernel-managed order and no clsact qdisc, unlike the legacy approach in Chapter 31 which used a clsact qdisc plus a single tc filter attached and ordered by hand."
   caption="Figure 36.1 — tcx: a kernel-ordered chain of programs via bpf_link, no clsact qdisc" %}

## What tcx changes

The program type is unchanged — it's still a `#[classifier]` returning a
`TC_ACT_*` verdict on a `TcContext`, exactly as in Chapter 31. What changes
is **how it attaches**:

- **No qdisc.** You don't `qdisc_add_clsact` anymore. tcx is a first-class
  attach point on the interface; the kernel handles the plumbing.
- **A link, not a filter.** Attaching returns a **`bpf_link`**. Hold it for
  as long as you want the program attached; **drop it and the program
  detaches**. No more orphaned filters surviving a crashed loader, and no
  manual teardown.
- **Ordered composition.** Multiple programs can attach to the same
  ingress/egress hook, and tcx runs them in a defined order you can control
  (attach *before* or *after* existing ones). Legacy clsact made this
  awkward; tcx makes it the model. This is what lets several eBPF tools
  (a tracer, a firewall, a load balancer) coexist on one interface.

Everything you learned about writing the classifier carries over; only the
attach call and its lifetime are new.

## How the code works

### The classifier (unchanged shape)

A minimal ingress counter — per-protocol packet counts in a map, always
`TC_ACT_OK`:

```rust
#[map] static PKTS: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);

#[classifier]
pub fn tcx_count(ctx: TcContext) -> i32 {
    let _ = count(&ctx);
    TC_ACT_OK                       // observe only; never disturb traffic
}
fn count(ctx: &TcContext) -> Result<(), ()> {
    let eth: EthHdr = ctx.load(0).map_err(|_| ())?;
    if eth.ether_type != EtherType::Ipv4 { return Ok(()); }
    let ip: Ipv4Hdr = ctx.load(EthHdr::LEN).map_err(|_| ())?;
    bump(&PKTS, ip.proto as u32, 1);
    Ok(())
}
```

### Attaching with tcx

Here's the whole difference from Chapter 31, side by side:

```rust
// Chapter 31 (legacy clsact):
//   let _ = tc::qdisc_add_clsact(&iface);                 // manage the qdisc yourself
//   prog.attach(&iface, TcAttachType::Egress)?;           // returns a filter handle

// Chapter 36 (tcx):
let prog: &mut SchedClassifier = ebpf.program_mut("tcx_count").unwrap().try_into()?;
prog.load()?;
let link_id = prog.attach(&iface, TcAttachType::Ingress)?;  // tcx on 6.6+, returns a link
// … keep the program attached for the process lifetime; the link owns it …
```

No qdisc call. The `attach` returns a **link handle**; the program stays
attached while that handle (and the loaded program) live, and detaches when
they're dropped at exit. To place this program relative to others on the
same hook, tcx takes ordering options (attach *before* or *after* a given
link) — the mechanism behind composing multiple eBPF programs on one
interface.

The rest is the familiar map drain: read `PKTS` on a timer and export
`ebpf_tcx_packets_total{proto}`.

## Build, deploy, observe

```bash
cd examples/36-tcx && ./demo.sh
```

The demo attaches the counter to the target's interface via tcx and drives
a little traffic. Two things are worth confirming live: the counts rise in
Grafana as before (the classifier logic is unchanged), and — unlike
Chapter 31 — there is **no clsact qdisc** on the interface, because tcx
didn't create one.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `sum by (direction) (rate(ebpf_tcx_packets_total[1m]))` — packets by direction at the tcx hook.

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`),
find the interface and compare what each tool sees:

```bash
[vm]$ ip -br link                       # find the NIC (e.g. enp1s0)
[vm]$ sudo bpftool net show             # tcx programs are listed under "tcx/ingress"
[vm]$ sudo tc filter show dev enp1s0 ingress   # EMPTY — tcx is not a tc filter
[vm]$ sudo tc qdisc show dev enp1s0     # no clsact qdisc was added
```

The telling contrast with Chapter 31: `bpftool net show` lists your program
under `tcx/ingress`, while `tc filter show … ingress` is **empty** and
there's **no clsact qdisc** — concrete proof that tcx is a different,
kernel-managed attach point, not a filter hanging off a qdisc.

## What you learned

- **`tcx`** (kernel 6.6+) is the modern tc attach: a `bpf_link` instead of a
  qdisc-hung filter, with **auto-detach on drop** and a **kernel-managed
  order** that lets multiple programs share a hook.
- The classifier code is identical to Chapter 31 — only the attach call and
  its lifetime change (no `qdisc_add_clsact`; `attach` returns a link).
- How to *see* the difference: `bpftool net show` lists `tcx/ingress` while
  `tc filter show` is empty and no clsact qdisc exists.

That closes the **Networking** part — from timing a connection through
dropping, capturing, load-balancing, testing, and now modern attachment.
Next is **Part 5, Security & LSM**, where eBPF stops reporting on the kernel
and starts making allow/deny decisions for it.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
