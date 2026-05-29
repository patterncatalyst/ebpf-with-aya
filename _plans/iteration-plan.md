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
| r12 | 23 | `profile` — sampling profiler, flame-graph-ready output | open |
| r13 | 24–25 | `memleak`, `biopattern` — allocation tracking, block I/O patterns | open |
| r14 | 26 | energy monitoring — eBPF offload of power/QoS feedback (modern theme) | open |

## Phase 5 — Networking

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r15 | 27–28 | `tcpconnlat`, `tcpstates` — connection latency, TCP state transitions (two-VM) |
| r16 | 29–30 | L7 tracing (http socket filters + syscall tracepoints); `sockops` |
| r17 | 31–32 | `tc` traffic control; `xdp` — first XDP program |
| r18 | 33–34 | `xdp tcpdump`; `xdp load balancer` (two-VM) |
| r19 | 35–36 | `xdp test` (BPF_PROG_TEST_RUN); `tcx` — modern TC attach |

## Phase 6 — Security & LSM

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r20 | 37 | `lsm connect` — deny operations via the LSM hook |
| r21 | 38–39 | hiding process/file information; `bpf_send_signal` to terminate malicious processes |
| r22 | 40 | sudo privilege-escalation via file-content manipulation (offense, lab-only, in the VM) |
| r23 | 41 | runtime hardening & security — telemetry, threat shielding (modern theme) |

## Phase 7 — Schedulers (`sched_ext`)

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r24 | 42 | `scx_simple` — a minimal BPF scheduler via `struct_ops`/`sched_ext` |
| r25 | 43 | `scx_nest` — a more realistic scheduling policy |

## Phase 8 — Application targets

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r26 | 44 | `nginx` — probing a real server (UBI nginx workload) |
| r27 | 45 | `postgres` — query/lock observation |

## Phase 9 — Advanced kernel surface (the 2024–2026 BPF feature set)

| r# | Chapter(s) | Topics |
|----|-----------|--------|
| r28 | 46–47 | `detach`; `syscall` programs |
| r29 | 48–49 | user ringbuf; userspace eBPF |
| r30 | 50–51 | `kfuncs`; `bpf token` |
| r31 | 52–53 | `bpf wq` (workqueues); `struct_ops` (general) |
| r32 | 54–55 | `dynptr`; `bpf arena`; `bpf iters` |

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
