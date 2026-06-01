# Product Requirements Document — eBPF with Aya on Fedora

> This PRD is the source of truth for what we're building. When scope
> creep tempts ("should we also cover X?"), the answer is whatever this
> PRD says. Updates to this file are commit-worthy events.

---

## 1. Summary

**One sentence:** A chapter-based, hands-on tutorial that teaches a
Rust programmer to write eBPF programs with Aya, deploy them safely to
a Fedora 44 KVM virtual machine, and observe their output in Grafana
via OpenTelemetry.

**One paragraph:** This tutorial takes a developer who has read *The
Rust Programming Language* and teaches them eBPF with [Aya](https://aya-rs.dev/)
— from a first tracepoint through XDP load balancers, `sched_ext`
schedulers, LSM security programs, and the 2024–2026 BPF feature set
(`struct_ops`, kfuncs, BPF arenas, BPF tokens, dynptr, BPF iterators).
It is structured as a lab: code is written on a Fedora 44 laptop but
deployed to a disposable Fedora 44 KVM guest, so a buggy program never
risks the working machine. Client load comes from Python 3.14 drivers
in Podman, and every program's measurements flow into Grafana (Tempo +
Mimir + Loki) through OpenTelemetry. It exists because eBPF learning
material is overwhelmingly C-first and Aya-the-Rust-way deserves a
single coherent, Fedora-native, observability-wired path.

---

## 2. Problem statement

**Who is the reader?** A working developer comfortable with Rust
(ownership, traits, async, Cargo) who wants to do kernel-level
observability, networking, or security work without writing C. They
run Fedora, use Podman daily, and have RustRover.

**What's their pain today?** eBPF tutorials are C-first; the Rust/Aya
material that exists is scattered across the Aya Book, blog posts, and
example repos, with no single path that also covers *how to run this
safely* (a VM, not your laptop) and *how to see the output* (a real
observability stack, not `println!`). Setup friction — toolchains,
BPF targets, linkers, kernel BTF — stops people before they write a
useful program.

**Why now?** Aya reached a stable, ergonomic 0.13 line; Fedora 44 ships
a BTF-enabled kernel making CO-RE turnkey; `grafana/otel-lgtm` makes a
full Grafana/Tempo/Mimir/Loki backend a single container; and the
modern BPF surface (`sched_ext`, `struct_ops`, kfuncs, arenas, tokens)
is now broad enough to be worth a long-form tutorial.

---

## 3. Goals and non-goals

### Goals

- A reader finishes Foundations (Ch 0–6) and can: provision a Fedora 44
  eBPF target VM, stand up the observability stack, install the Aya
  toolchain, and build/deploy/observe a first program — without
  consulting other docs.
- Each program chapter ships a runnable Aya project plus a `demo.sh`
  that builds, deploys to the VM, drives load, and shows output in
  Grafana.
- The tutorial covers the full topic list (§5) across iterations,
  starting with a libbpf/libbpf-rs warm-up and using Aya as the primary
  toolkit thereafter.
- Modern themes (BPF CO-RE, L3AF zero-downtime upgrades, AI/GPU
  offloading, power management, runtime hardening) are woven in where
  relevant, not bolted on.
- Every claim is tracked in the reconciliation plan and only marked
  `verified` after a real Fedora 44 run.

### Non-goals

- **Not a Rust tutorial.** *The Rust Programming Language* is assumed.
- **Not a kernel-internals course.** eBPF concepts are taught
  (Chapter 5); kernel subsystem internals are referenced, not derived.
- **Not production-grade deployment guidance** (orchestration at scale,
  signing, fleet rollout) beyond what L3AF/operating chapters introduce.
- **Not Windows.** macOS is acknowledged for the client/stack pieces
  only; the KVM target is Linux.
- **No copied/ported code** — insight from anywhere, but the code we
  ship is original and clearly licensed (see `CONTRIBUTING.md`).

---

## 4. Audience

- **Primary:** Fedora 44 developers fluent in Rust who want eBPF
  without C.
- **Secondary:** developers on Fedora derivatives (RHEL/Rocky/Alma 10)
  for the host role; Aya-curious engineers wanting a wired-up lab.
- **Not served:** Rust beginners, complete kernel newcomers, Windows
  users.

---

## 5. Scope and chapter outline

The tutorial is grouped into parts; chapters map to `_docs/NN-*.md`.
The Foundations part is fully written in r1.0; later parts are mapped
in [`_plans/iteration-plan.md`](./_plans/iteration-plan.md).

| Part | Chapters | Topics |
|------|----------|--------|
| Foundations | 0–6 | outline, prerequisites, lab VMs + networking, observability stack, Rust/Aya toolchain, eBPF concepts, hello-world |
| Tracing the kernel | 7–12 | kprobe+unlink, fentry+unlink, opensnoop, sigsnoop, execsnoop, exitsnoop |
| User-space & language probing | 13–20 | uprobe+bashreadline, bootstrap (java/python targets), uprobe rust, btf uprobe, sslsniff, trace goroutine states, funclatency, javagc |
| Performance & resources | 21–26 | runqlat, hardirqs, profile, memleak, biopattern, energy monitoring |
| Networking | 27–36 | tcpconnlat, tcpstates, L7 http socket filters, sockops, tc, xdp, xdp tcpdump, xdp load balancer, xdp test, tcx |
| Security & LSM | 37–42 | lsm connect, bpf_send_signal kill, hiding process info, LSM file protection, sudo priv-esc demo, security sensor/telemetry |
| Schedulers (sched_ext) | 43–44 | scx_simple, scx_nest |
| Application targets | 45–47 | nginx, three-signal capstone (Java + Python, OTel/OBI), postgres |
| Advanced kernel surface | 48–57 | detach, syscall, user ringbuf, userspace ebpf, kfuncs, bpf token, bpf wq, struct_ops, dynptr, bpf arena, bpf iters |
| Operating eBPF | 58–63 | CO-RE deep dive, L3AF zero-downtime upgrades, AI/GPU offloading, power management (RAPL + eBPF attribution), signal correlation (Tempo/Mimir/spans), end-to-end capstone (one request, every layer) |
| Field guide *(optional)* | 64–66 | bpftrace from Python (NDJSON), bpftool from Python (JSON inventory/audit), BCC tools tour from Python |
| Retrospective | 67 | the whole arc from kprobe to fleet; what held constant; where eBPF & Aya go next |

---

## 6. Runnable examples

**Yes** — every hands-on chapter has an `examples/NN-name/` directory:

```
examples/NN-name/
├── README.md       — narrated walkthrough (source of truth)
├── demo.sh         — build -> deploy to VM -> drive load -> observe (also the test)
├── Cargo.toml + crates/  — the Aya workspace (where applicable)
└── client/         — Python 3.14 load driver (where applicable)
```

`demo.sh` conventions: `set -euo pipefail`; `127.0.0.1` not
`localhost`; wait-for-HTTP not `sleep`; `trap` cleanup on exit;
`:Z` on mounts; deploy to the target VM via `scripts/lab/`.

Languages/tools: **Rust 1.96.0** + Aya (`aya` 0.13.x, `aya-ebpf` 0.1.x)
for programs; **Python 3.14** in Podman for clients; `bpftool` /
`bpftrace` / `bcc-tools` (Fedora repos) for cross-checks;
`grafana/otel-lgtm` for the backend.

**Container & target policy.** Everything user-space runs in a
container (Podman / podman-compose) with **multi-stage, UBI-based**
Containerfiles — the sole exception being the privileged Aya loader,
which runs as a binary on the target VM. Observed application targets
are pinned: **Java 25 (LTS) + Quarkus LTS 3.33**, and **Python 3.14 +
FastAPI**, both containerized. The tutorial covers **observing a
containerized target** — host-vs-container PID and path resolution —
and **crun 1.27.1** (Fedora's default OCI runtime) including its eBPF
and SELinux behavior. Architecture diagrams are authored in
**Excalidraw** (`.excalidraw` source + exported `.svg`, embedded via the
`excalidraw.html` include).

---

## 7. Lab architecture

- **Host:** Fedora 44 laptop. Edits (RustRover), builds (Cargo), runs
  clients (Podman) and the observability stack (Podman Compose). Never
  the eBPF target.
- **Target VM:** `ebpf-target`, Fedora 44 under KVM/libvirt, provisioned
  by script from a Cloud Base image + cloud-init. Disposable.
- **Peer VM (optional):** `ebpf-peer`, second Fedora 44 guest on the
  same libvirt network, for two-host networking chapters.
- **Deploy model:** build once on the host → `scp` the single
  self-contained Aya binary to the guest → run under `sudo`. The guest
  needs only a kernel, never Rust.

---

## 8. Reference material

*Learning eBPF* (Liz Rice); [ebpf.io/get-started](https://ebpf.io/get-started);
*BPF Performance Tools* (Brendan Gregg); the Aya Book; Isovalent/Cilium
material; *Oxidizing eBPF*; Luca Palmieri and *Zero To Production in
Rust* ([zero2prod.com](https://www.zero2prod.com)) for Rust
project/application patterns.

---

## 9. Project state

- **Iteration:** r1.0 — scaffold + Foundations (Ch 0–6) + full roadmap.
- **Verified facts:** 0 (nothing has been run on Fedora 44 yet; the lab
  to verify against is itself part of what r1.0 delivers). All claims
  `unverified` — see [`_plans/reconciliation-plan.md`](./_plans/reconciliation-plan.md).
- **Next:** verify Foundations on real hardware, then Part "Tracing the
  kernel" (kprobe+unlink) per the iteration plan.
