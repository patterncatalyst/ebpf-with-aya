---
title: Iteration roadmap
description: How the full topic list maps onto iterations, from the r1.0 scaffold to the complete tutorial, with the per-iteration verification rhythm.
render_with_liquid: false
---

This is the roadmap from the r1.0 scaffold to a complete tutorial. Each
iteration ships as `ebpf-with-aya-rNN.x.tar.gz` per the cadence in
`CONTRIBUTING.md`, extracts in place, and is committed as
`chore: archive rNN — <summary>`. The reconciliation plan tracks
verification state for every claim across all iterations.

Iteration boundaries are flexible; **part boundaries are stable**. A
dense iteration may split into sub-iterations (`rNNa`, `rNNb`); a light
one may merge with its neighbour.

## Phase 1 — Foundations

| r# | Deliverables | Status |
|----|--------------|--------|
| **r1.0** | Site scaffold (config, layouts, includes, CSS, index, Pages workflow); README, PRD, CONTRIBUTING; onboarding docs; reconciliation + iteration plans; **Chapters 0–6** (outline, prerequisites, lab setup, observability stack, toolchain, concepts, hello-world); lab scripts; `examples/03-observability-stack/` and `examples/06-hello-world/` | shipped (unverified) |
| r1.1 | Verify Foundations on real Fedora 44; fix whatever the first `cargo build` / `./demo.sh` / VM provision surfaces; promote rows | open |
| r6.1 | **site:** two-level Part/Chapter navigation (homepage shows Part cards; each Part page lists its chapters) | shipped (unverified) |
| r7.1 | **conventions:** container policy (multi-stage UBI, all user space in Podman), version pins (Java 25/Quarkus 3.33, Python 3.14/FastAPI, crun 1.27.1), first Excalidraw diagram (lab topology) | shipped (unverified) |
| r14.1 | **diagrams:** 19 Excalidraw+SVG diagrams (Tier 1–3) embedded across Ch 3–26; spec-based generator; networking diagrams deferred to r15+ | shipped (unverified) |

## Phase 2 — Tracing the kernel (Part: kprobes, fentry, tracepoints)

| r# | Chapter(s) | Topics | Status |
|----|-----------|--------|--------|
| r02 | 7 | `kprobe` + `unlink` — first kprobe, read syscall args | **shipped (unverified)** |
| r03 | 8 | `fentry` + `unlink` — BTF-based, lower overhead than kprobes; compare | **shipped (unverified)** |
| r04 | 9–10 | `opensnoop`, `sigsnoop` — ring buffers, per-event records | **shipped (unverified)** |
| r05 | 11–12 | `execsnoop`, `exitsnoop` — process lifecycle tracepoints | **shipped (unverified)** |

## Phase 3 — User-space & language probing (Part: uprobes, USDT, runtimes)

| r# | Chapter(s) | Topics | Status |
|----|-----------|--------|--------|
| r06 | 13–14 | `uprobe` + `bashreadline`; `uprobe rust` (probe a Rust binary's symbols) | **shipped (unverified)** |
| r07 | 15 | `btf uprobe` — BTF-assisted user probes | **shipped (unverified)** |
| r08 | 16 | bootstrap for user-space targets — Java and Python target examples | **shipped (unverified)** |
| r09 | 17–18 | `sslsniff`; `funclatency` — uprobe-based latency histograms | **shipped (unverified)** |
| r10 | 19–20 | trace goroutine states; `javagc` — runtime-aware probing | **shipped (unverified)** |

## Phase 4 — Performance & resources

| r# | Chapter(s) | Topics | Status |
|----|-----------|--------|--------|
| r11 | 21–22 | `runqlat`, `hardirqs` — scheduling latency, IRQ timing | **shipped (unverified)** |
| r12 | 23 | `profile` — sampling profiler, flame-graph-ready output | **shipped (unverified)** |
| r13 | 24–25 | `memleak`, `biopattern` — allocation tracking, block I/O patterns | **shipped (unverified)** |
| r14 | 26 | energy monitoring — eBPF offload of power/QoS feedback (modern theme) | **shipped (unverified)** |

## Phase 5 — Networking

| r# | Chapter(s) | Topics | Status |
|----|-----------|--------|--------|
| r15 | 27–28 | `tcpconnlat`, `tcpstates` — connection latency, TCP state transitions (two-VM) | **shipped (unverified)** |
| r16 | 29–30 | L7 tracing (http socket filters + syscall tracepoints); `sockops` | **shipped (unverified)** |
| r17 | 31–32 | `tc` traffic control; `xdp` — first XDP program | **shipped (unverified)** |
| r18 | 33–34 | `xdp tcpdump`; `xdp load balancer` (two-VM) | **shipped (unverified)** |
| r19 | 35–36 | `xdp test` (BPF_PROG_TEST_RUN); `tcx` — modern TC attach | **shipped (unverified)** |

## Phase 6 — Security & LSM

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r20 | 37–38 | `lsm connect` — deny via LSM (cgroup-scoped); `signal kill` — bpf_send_signal to terminate processes | **shipped (unverified)** |
| r21 | 39–40 | hiding process/file information (lab-only offense); LSM file/tamper protection | **shipped (unverified)** |
| r22 | 41–42 | sudo privilege-escalation (offense, lab-only); security sensor — telemetry + shielding (modern theme) | **shipped (unverified)** |

## Phase 7 — Schedulers (`sched_ext`)

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r23 | 43 | `scx_simple` — minimal scheduler via `struct_ops`/`sched_ext` (model + run + Aya observer) | **shipped (unverified)** |
| r24 | 44 | `scx_nest`-style policy — keeping work on warm cores (more realistic) | **shipped (unverified)** |

## Phase 8 — Application targets

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r25 | 45 | `nginx` — uprobe per-request latency on a containerized server | **shipped (unverified)** |
| r26 | 46 | capstone — three signals tied together (Java + Python, OTel/OBI) | **shipped (unverified)** |
| r27 | 47 | `postgres` — query latency + lock waits (multi-process uprobes, USDT) | **shipped (unverified)** |

## Phase 9 — Advanced kernel surface (the 2024–2026 BPF feature set)

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r28 | 48 | `detach`/pinning — outliving the loader | **shipped (unverified)** |
| r29 | 49 | `syscall` programs — loader programs & light skeletons | **shipped (unverified)** |
| r30 | 50 | user ring buffer (user → BPF) | **shipped (unverified)** |
| r31 | 51 | userspace eBPF (rbpf, bpftime) | **shipped (unverified)** |
| r32 | 52 | `kfuncs` — typed kernel calls, KF_ACQUIRE/RELEASE | **shipped (unverified)** |
| r33 | 53 | `bpf token` — delegating BPF into containers (kernel 6.9) | **shipped (unverified)** |
| r34 | 54 | timers & workqueues — deferred in-kernel work | **shipped (unverified)** |
| r35 | 55 | `struct_ops` (general) — BPF implements a kernel vtable | **shipped (unverified)** |
| r36 | 56 | `dynptr` & `bpf arena` — flexible/shared memory | **shipped (unverified)** |
| r37 | 57 | `bpf iterators` — walk a kernel set, emit like a file | **shipped (unverified)** — Part 8 complete |
| r38 | 58 | CO-RE deep-dive — compile once, run everywhere | **shipped (unverified)** — opens Part 9 |

## Phase 10 — Operating eBPF

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r33 | 56 | BPF CO-RE deep dive — relocations, portability across kernels |
| r34 | 57 | L3AF — managing and zero-downtime-upgrading eBPF programs |
| r35 | 58 | AI & GPU offloading — tracing data flows in AI/GPU clusters (modern theme) |
| r36 | 59–60 | power management recap; where to go next + project close-out (prd-reconciliation) |

## Within-iteration verification rhythm

For any iteration shipping an Aya program:

1. Tarball delivered with the example dir, chapter prose, and
   reconciliation rows marked `unverified` / `in flight`.
2. Extract into the working copy, push, `gh run watch` confirms the
   site build is green.
3. `cd examples/NN-name/ && ./demo.sh`; share output.
4. **Pass** → next iteration's first move flips the row to
   `verified (Fedora 44)`. **Fail** → diagnose from output, fix in
   `rNNa`, re-run.

This honors "tested code first, then prose": prose ships alongside the
code but does not claim `verified` until a real Fedora 44 run passes.

## Flex points

- The libbpf/libbpf-rs warm-up lives inside Chapter 6 rather than as
  its own iteration; Aya is the toolkit everywhere after.
- Two-VM chapters (Phase 5 networking, the load balancer) assume the
  optional `ebpf-peer` guest from Chapter 2.
- Modern-theme chapters (energy monitoring, runtime hardening, AI/GPU
  offloading, L3AF) are placed where they reinforce a concrete program,
  not isolated at the end.
- The offense chapter (sudo priv-esc) runs only inside the disposable
  target VM and is framed defensively.
