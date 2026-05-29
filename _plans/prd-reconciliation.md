---
title: PRD reconciliation
description: What we set out to build per the PRD, what shipped, and where intent and delivery diverged. Filled in as the project progresses; completed at close.
render_with_liquid: false
---

> Counterpart to `reconciliation-plan.md`. That file tracks
> claim-level verification; this one tracks the high-level question:
> what did the PRD say we'd build, what did we actually ship, and where
> did intent and delivery diverge — with rationale for each divergence.
>
> This is a living stub. It gets a real entry at the end of each phase
> and a full pass at project close.

---

## Status: in progress (r1.0)

Nothing has diverged from the PRD yet — r1.0 ships exactly the
Foundations scope the PRD's §5 outline calls for. Divergences will be
recorded here as they happen.

## Goals — planned vs shipped

| PRD goal (paraphrased) | Status | Notes |
|---|---|---|
| Reader finishes Foundations and can provision the lab, stand up the stack, install the toolchain, and build/deploy/observe a first program | shipped, unverified | Chapters 0–6 + the two examples deliver this; not yet run on Fedora 44 |
| Each program chapter ships a runnable Aya project + `demo.sh` | on track | Pattern established in Ch 3 and Ch 6; later chapters follow |
| Cover the full topic list across iterations, libbpf warm-up then Aya | on track | Mapped in the iteration roadmap; libbpf warm-up folded into Ch 6 |
| Modern themes woven in where relevant | planned | Placed in the roadmap (energy, hardening, AI/GPU, L3AF) next to concrete programs |
| Every claim tracked; verified only after real run | holding | All r1.0 claims `unverified` by design |

## Non-goals — held

| PRD non-goal | Status |
|---|---|
| Not a Rust tutorial | held |
| Not a kernel-internals course | held |
| Not production deployment at scale | held |
| Not Windows; macOS client/stack only | held |
| No code from China/Russia/North Korea/Iran repos | held — policy in CONTRIBUTING.md |

## Divergences

*(none yet)*

## Open decisions to revisit

- Whether to keep the `aya-build` `build.rs` approach in examples or
  switch to whatever the `aya-template` settles on, once verified on
  hardware (Ch 6 risk #1 in the reconciliation plan).
- Whether two-VM networking chapters need the optional isolated libvirt
  network by default, or keep the `default` NAT network as baseline.
