---
title: Tutorial outline
order: 0
part: Foundations
description: What this tutorial covers, how the chapters are grouped, and the order in which they build on each other.
duration: 6 minutes
---

This tutorial takes a developer who already knows Rust (you have read
*The Rust Programming Language*) and teaches them to write **eBPF
programs in Rust with [Aya](https://aya-rs.dev/)** — from a first
"hello world" probe through XDP load balancers, `sched_ext`
schedulers, LSM security programs, and the newest kernel surfaces
like `struct_ops`, kfuncs, BPF arenas, and BPF tokens.

It is deliberately structured as a **lab**. You write code on a
Fedora 44 laptop, but you *never load eBPF into your laptop's
kernel*. Instead you deploy each program to a disposable Fedora 44
virtual machine running under KVM/QEMU, so a program that panics the
verifier — or worse, the kernel — costs you a `virsh destroy` and a
re-provision, not your working machine. Client load comes from
Python 3.14 drivers in Podman, and every program's output is wired
into Grafana through OpenTelemetry so you can *see* what your probe
measured.

## How the chapters are grouped

The tutorial is organized into parts. The **Foundations** part
(Chapters 0–6) is the setup you do once; everything after it is a
self-contained program you build, deploy, and observe. Later parts
can be read roughly in order, but each program chapter stands on its
own once the lab exists.

| Part | Theme | Chapters |
|------|-------|----------|
| **Foundations** | Lab, toolchain, first program | 0–6 |
| **Tracing the kernel** | kprobes, fentry, tracepoints | 7–12 |
| **User-space &amp; language probing** | uprobes, USDT, runtimes | 13–20 |
| **Performance &amp; resources** | scheduling, IRQs, memory, I/O, power | 21–26 |
| **Networking** | sockets, TC, XDP, L7 | 27–36 |
| **Security &amp; LSM** | enforcement, offense, defense | 37–42 |
| **Schedulers (`sched_ext`)** | writing BPF schedulers | 43–44 |
| **Application targets** | real workloads | 45–47 |
| **Advanced kernel surface** | the 2024–2026 BPF feature set | 48–57 |
| **Operating eBPF** | CO-RE, zero-downtime, offload, power, signal correlation, capstone | 58–63 |
| **Field guide** *(optional)* | `bpftrace`, `bpftool`, and the BCC tools, driven from Python | 64–66 |
| **Retrospective** | the whole arc, and where eBPF &amp; Aya go next | 67 |

## The Foundations chapters (do these first, in order)

These six chapters are the floor. Nothing in the later parts works
until you have a lab that builds, deploys, and reports.

| Ch | Title | What you get |
|----|-------|--------------|
| 1 | Prerequisites | Hardware floor, Fedora 44, KVM/QEMU/libvirt, what to have installed |
| 2 | Lab setup: the eBPF target VM(s) | A scripted Fedora 44 guest under libvirt; a second guest + the network between them for two-host tests |
| 3 | The observability stack | `grafana/otel-lgtm` under Podman Compose; a Python 3.14 OTLP client; how Rust user space exports to it |
| 4 | The Rust + Aya toolchain | `rustup` (Rust 1.96.0), the BPF target, `bpf-linker`, `cargo-generate` + `aya-template`, RustRover, kernel tools from Fedora repos |
| 5 | eBPF concepts and tools | Programs, maps, the verifier, BTF, CO-RE, and how `bpftool`/`bpftrace` fit in |
| 6 | Hello, eBPF | A first Aya program (with a short libbpf / libbpf-rs warm-up), deployed to the VM, output in Grafana |

## What comes after Foundations

Each later chapter follows the same shape: a short concept section, a
runnable Aya project under `examples/`, a `demo.sh` that builds it,
ships it to the target VM, runs it, drives load, and shows where the
output appears in Grafana. The full chapter list — including the
exact programs (`opensnoop`, `execsnoop`, `tcpconnlat`, `xdp`
load balancer, `scx_nest`, `sslsniff`, `struct_ops`, and the rest) unfolds across the parts ahead.

We start the eBPF work with **libbpf and libbpf-rs** for a single
chapter so the C-and-CO-RE mental model is concrete, then move to
**Aya** as the primary toolkit for everything else.

## Conventions used throughout

- All host commands target **Fedora 44** with **rootless Podman** and
  **libvirt/KVM**. Where a command must run *inside the target VM*
  rather than on the laptop, the prose says so explicitly and the
  prompt is shown as `[vm]$`.
- Commands you paste are written as **single lines** (semicolons and
  `\` continuations rather than multi-line blocks) because multi-line
  pastes misbehave in zsh with bracketed-paste and autosuggest. Any
  multi-step procedure ships as a **script in the iteration tarball**,
  not as a block to paste by hand.
- Test scripts use **`127.0.0.1`**, never `localhost` — modern `curl`
  prefers IPv6 `::1` and many runtimes bind IPv4 only.
- Bind mounts carry the **`:Z`** SELinux relabel suffix. It is correct
  on Fedora and a harmless no-op elsewhere.
- Container images are **UBI-based** (`registry.access.redhat.com/ubi9/...`)
  and pull without a Red Hat subscription.
- Kernel tooling — `bpftool`, `bpftrace`, `bcc`, `perf` — is installed
  from **Fedora/Red Hat repositories** via `dnf`, never from third-party
  binaries.
- Every technical claim starts life as <span class="status status--unverified">unverified</span>
  in the [reconciliation plan]({{ "/plans/reconciliation-plan/" | relative_url }})
  and is only promoted to <span class="status status--verified">verified (Fedora 44)</span>
  after it has been run end-to-end on real hardware. If a chapter
  hasn't been verified yet, its claims are marked as such.

## Prerequisite knowledge (not taught here)

- **Rust** at the level of *The Rust Programming Language* — ownership,
  traits, `Result`, `async`/`await`, Cargo. This is not a Rust course.
- Comfort with the **Linux command line** and a shell (the tutorial
  assumes zsh, but bash is fine).
- A **paragraph-level** idea of what the Linux kernel does: syscalls,
  processes, network packets. eBPF concepts themselves are taught in
  Chapter 5.

## Reference material

The tutorial leans on, and frequently points at, this canonical set:

- *Learning eBPF* — Liz Rice
- [ebpf.io/get-started](https://ebpf.io/get-started)
- *BPF Performance Tools* — Brendan Gregg
- The **Aya Book** and Aya API docs
- Articles by **Luca Palmieri** and the *Zero To Production in Rust*
  book ([zero2prod.com](https://www.zero2prod.com)) for Rust
  application and project-structure patterns
- Material from **Isovalent/Cilium** and the broader *Oxidizing eBPF*
  community

> **A note on sources** — We take *insight* freely from the whole eBPF
> community, but the code we ship is our own: we don't copy or port
> code line-for-line from other repositories, and anything borrowed
> carries a clearly compatible license. See
> [`CONTRIBUTING.md`](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/blob/main/CONTRIBUTING.md)
> for the full provenance policy.

Ready? [Start with Chapter 1: Prerequisites →]({{ "/docs/01-prerequisites/" | relative_url }})
