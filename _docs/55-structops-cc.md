---
title: "struct_ops: BPF that implements the kernel"
order: 55
part: Advanced kernel surface
description: "Most BPF observes or filters at a hook the kernel calls. struct_ops inverts that: the kernel defines an interface — a struct of function pointers — and BPF supplies the whole implementation, which the kernel then calls into like a built-in module. Understand the vtable model behind Part 6's schedulers, write a minimal TCP congestion-control algorithm in BPF, and see the same mechanism reused across the kernel."
duration: 45 minutes
---

Everything in this book so far has been BPF *reacting*: the kernel hits a hook
it owns — a kprobe, a tracepoint, an XDP point — and calls your one program.
**struct_ops** turns that relationship inside out. Here the kernel defines a
whole **interface** — a `struct` full of function pointers, a vtable — and BPF
supplies the *entire implementation*. Once registered, the kernel calls into
your BPF programs at each of the interface's call sites exactly as if you'd
shipped a kernel module implementing that policy. This is the mechanism beneath
Part 6's schedulers (`sched_ext` is "just" a big struct_ops), and it's how BPF
became a way to write pluggable kernel subsystems without writing a module.
This chapter generalizes it and grounds it in the original, most approachable
user: **TCP congestion control**.

The code is in `examples/55-structops-cc/`. `./demo.sh` compiles a minimal
congestion-control algorithm, registers it, and shows the kernel offering it
alongside cubic and reno; the `README.md` has the details.

{% include excalidraw.html
   file="structops"
   alt="struct_ops: BPF implements a kernel interface (a vtable), and the kernel calls in. On the left, BPF programs in sections struct_ops/ssthresh, struct_ops/cong_avoid, and struct_ops/undo_cwnd fill the slots of a STRUCT_OPS map holding a struct tcp_congestion_ops, which is registered via a link. On the right, the TCP stack's call sites invoke them: on ACK it calls cong_avoid, on loss it calls ssthresh, and the algorithm is selected via sysctl or setsockopt. The kernel calls your BPF. The same mechanism powers sched_ext (Part 6), HID-BPF, bpf Qdisc, and FUSE — BPF as a pluggable kernel module."
   caption="Figure 55.1 — BPF fills a kernel-defined vtable; the kernel calls into it at the interface's call sites" %}

## The inversion

A normal BPF attachment answers the question "what should run when the kernel
reaches *this* point?" — one program, one hook. A kernel **interface** asks a
bigger question: "who implements *this whole policy*?" TCP congestion control
is the classic example. The kernel doesn't have one congestion-control hook; it
has a `struct tcp_congestion_ops` — a contract of operations the TCP stack
calls at different moments:

- `.ssthresh(sk)` — on loss, return the new slow-start threshold.
- `.cong_avoid(sk, ack, acked)` — on each ACK, grow the congestion window.
- `.undo_cwnd(sk)` — after a spurious loss, what window to restore.
- plus optional hooks (`.init`, `.set_state`, `.cwnd_event`, `.pkts_acked`…).

Historically you implemented that contract by writing a kernel module (cubic,
bbr, reno) and registering it. struct_ops lets you implement it **in BPF**:
provide a program for each function pointer, register the filled-in struct, and
the TCP stack calls your programs at exactly those moments. As the feature's
authors noted, this brought the *fast turnaround* of user-space experimentation
to something that absolutely must run in-kernel — test a new congestion
algorithm in production without shipping a kernel, without leaving the kernel.

## How struct_ops works

The mechanism rests on three things this book has already built up — BTF, the
BPF trampoline, and CO-RE — and looks like this:

- A special map of type **`BPF_MAP_TYPE_STRUCT_OPS`** holds *the struct itself*
  as its value. You declare it as a global in `SEC(".struct_ops.link")`, with
  each function-pointer field assigned a BPF program:

  ```c
  SEC(".struct_ops.link")
  struct tcp_congestion_ops bpf_reno = {
      .ssthresh   = (void *)cc_ssthresh,
      .cong_avoid = (void *)cc_cong_avoid,
      .undo_cwnd  = (void *)cc_undo_cwnd,
      .name       = "bpf_reno",
  };
  ```

- Each implementation is a program in **`SEC("struct_ops/<anything>")`** with
  the signature the kernel expects for that slot.
- At load, the loader uses **BTF reflection** to match your struct and its
  members to the kernel's real `tcp_congestion_ops` — by member **name, BTF
  kind, and size**. If a field's name or type doesn't line up with the kernel's
  definition, it's rejected. This is why the same machinery works for *any*
  kernel struct_ops interface without per-interface loader code: it's reflection
  over BTF, not hardcoding.
- **Registering** the map (via a struct_ops link) installs the implementation;
  the algorithm now appears in the kernel's list and can be selected. Closing
  the link **unregisters** it. The verifier specifically permits these programs
  to *write* a few kernel fields they need to (e.g. parts of `tcp_sock`), which
  ordinary tracing programs may not — implementing policy means mutating state,
  carefully and within bounds the verifier checks.

## A minimal congestion-control algorithm

The worked example is a small Reno-style algorithm. The three required ops, in
BPF:

```c
SEC("struct_ops/cc_ssthresh")
__u32 BPF_PROG(cc_ssthresh, struct sock *sk) {
    const struct tcp_sock *tp = tcp_sk(sk);
    return max(tp->snd_cwnd >> 1, 2U);          /* halve the window on loss */
}

SEC("struct_ops/cc_cong_avoid")
void BPF_PROG(cc_cong_avoid, struct sock *sk, __u32 ack, __u32 acked) {
    struct tcp_sock *tp = tcp_sk(sk);
    if (tp->snd_cwnd < tp->snd_ssthresh)
        tcp_slow_start(tp, acked);               /* exponential, below ssthresh */
    else
        tcp_cong_avoid_ai(tp, tp->snd_cwnd, acked); /* additive increase above  */
}

SEC("struct_ops/cc_undo_cwnd")
__u32 BPF_PROG(cc_undo_cwnd, struct sock *sk) {
    const struct tcp_sock *tp = tcp_sk(sk);
    return max(tp->snd_cwnd, tp->prior_cwnd);    /* restore after spurious loss */
}
```

Reading it as the TCP stack will run it:

- On every ACK the stack calls **`cc_cong_avoid`**, handing us the socket and
  how many packets were acked. Below the slow-start threshold we grow the
  window exponentially (`tcp_slow_start`); above it, additively
  (`tcp_cong_avoid_ai`). Those two are kernel functions the CC struct_ops
  interface makes callable — the same way kfuncs (Chapter 52) expose typed
  kernel functions to BPF, the congestion-control interface exposes its building
  blocks.
- On loss the stack calls **`cc_ssthresh`**; we return half the window — textbook
  Reno multiplicative decrease.
- We're allowed to *write* `tcp_sock` fields here (via those helpers) precisely
  because we're implementing the interface, and the verifier knows it.

Register it, and it joins the kernel's roster:

```bash
[vm]$ sysctl net.ipv4.tcp_available_congestion_control   # ... cubic reno bpf_reno
[vm]$ sysctl -w net.ipv4.tcp_congestion_control=bpf_reno  # make it the default
```

Your BPF is now the machine's congestion-control algorithm. Every TCP
connection that selects it runs *your code* on every ACK and every loss.

## The same mechanism, everywhere

Once you see the pattern — *BPF fills a kernel-defined vtable, registered as a
struct_ops map* — you recognize it across the modern kernel:

- **sched_ext** (Part 6): `sched_ext_ops` is a struct_ops with dozens of slots
  (`.enqueue`, `.dispatch`, `.select_cpu`…). The schedulers you wrote there were
  struct_ops all along.
- **HID-BPF**: `hid_bpf_ops` to fix up input-device reports.
- **bpf Qdisc**: implement a queueing discipline in BPF.
- **FUSE-BPF**: accelerate filesystem operations.

Each is the identical machinery with a different kernel struct; learning it once
unlocks all of them.

## Where Aya fits

struct_ops authoring in aya-ebpf is **emerging** — the same frontier the
sched_ext chapters flagged — so the canonical implementation lives in
`examples/55-structops-cc/reference/cc.bpf.c`, and the example registers it the
production way, with **`bpftool struct_ops register`**, which needs no Aya at
all. An Aya rendering is included to read. The conceptual takeaway doesn't
depend on the tooling: struct_ops is reflection over BTF, so any loader — libbpf,
bpftool, eventually Aya — installs it the same way.

## Build, deploy, observe

```bash
cd examples/55-structops-cc && ./demo.sh
```

The demo generates `vmlinux.h`, compiles the CC algorithm, and registers it with
`bpftool`. There's no Grafana panel — like the BPF token, this is a
control-plane act, not a data source — so you observe it where the kernel
exposes it: the algorithm appearing in `tcp_available_congestion_control`, and,
if you select it and drive traffic, in per-socket stats.

## Cross-check

```bash
[vm]$ sysctl net.ipv4.tcp_available_congestion_control   # bpf_reno listed
[vm]$ sudo bpftool struct_ops show                        # the registered struct_ops
[vm]$ sudo bpftool struct_ops dump name bpf_reno          # its programs and fields
[vm]$ ss -ti | grep -A1 bpf_reno                          # connections using it
```

`bpftool struct_ops show` listing your algorithm, and `ss -ti` reporting it on a
live connection, are the proof the kernel adopted your BPF as a first-class
implementation of its own interface.

## What you learned

- **struct_ops** inverts the usual model: the kernel defines an **interface** (a
  struct of function pointers) and BPF supplies the whole implementation, which
  the kernel calls at the interface's call sites — BPF as a pluggable kernel
  module.
- It works by **BTF reflection**: a `BPF_MAP_TYPE_STRUCT_OPS` map holds the
  struct, `SEC("struct_ops/…")` programs fill its slots, members are matched to
  the kernel's by name/kind/size, and registering a link installs it; the
  verifier lets these programs write the fields the interface requires.
- The original user is **TCP congestion control**, and the *same* mechanism
  powers **sched_ext**, **HID-BPF**, **bpf Qdisc**, and **FUSE** — learn it once,
  recognize it everywhere.

Next, Chapter 56 looks at **dynptrs and BPF arenas** — the modern way BPF
handles variable-length data and shares large memory regions.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that the CC algorithm compiles against this
kernel's `vmlinux.h`, that `bpftool struct_ops register` installs it and it
appears in `tcp_available_congestion_control`, that selecting it and driving
traffic shows it in `ss -ti`; the exact `tcp_sock` field names
(`snd_cwnd`/`snd_ssthresh`/`prior_cwnd`) for this kernel; and treat the aya-ebpf
struct_ops rendering as emerging.*
