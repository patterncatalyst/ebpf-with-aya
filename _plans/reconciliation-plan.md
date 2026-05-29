---
title: Reconciliation plan
description: What in this tutorial is verified on real Fedora 44 hardware, what is in flight, and what still needs validation.
render_with_liquid: false
---

This document tracks the **gap between what the tutorial claims and
what has been verified end-to-end** on real hardware. It is the single
authoritative list of things to check before any part is declared
done.

## How to use this document

- New content making a verifiable claim → add a row as `unverified`.
- You actually test a claim on Fedora 44 → promote to
  `verified (Fedora 44)`, noting which platform.
- Actively debugging → `in flight` with a note on what's blocking.
- Deliberately not testing this iteration → `out of scope` with the
  reason.

**Default state for new claims is `unverified`.** Promotion to
`verified` requires a real test run by a human, recorded here. An AI
assistant must not self-promote.

## Conventions

- `verified (Fedora 44)` — run end-to-end on Fedora 44 with the exact
  commands shown. This is the canonical primary-platform marker.
- `in flight` — actively being worked on.
- `unverified` — taken from sources/docs, not yet run here.
- `out of scope` — deliberately not verified this iteration.

> **r1.0 honesty note.** This iteration delivers the *lab that
> verification will run against*, so **nothing is verified yet** — there
> was no Fedora 44 target available while authoring. Every row below is
> `unverified`. The first real-hardware pass (r1.1) is expected to
> surface concrete fixes, especially in the Aya build wiring (Chapter 6)
> and the exact Fedora Cloud image filename (Chapter 2). That is the
> process working as intended, not a defect.

## A. Pinned tool versions

The versions this tutorial is written against. Every later claim
implicitly assumes these.

| Status | Tool | Version | Where pinned | Notes |
|--------|------|---------|--------------|-------|
| unverified | Fedora (host + guest) | 44 | Ch 1–2 | Released ~Apr 2026; BTF-enabled stock kernel |
| unverified | Rust toolchain | 1.96.0 | Ch 4, `rust-toolchain.toml` | **Beta at authoring (late May 2026); goes stable ~2026-06-05.** Stable today is 1.95.0 — use it if 1.96.0 stable isn't out yet |
| unverified | Rust nightly | latest + `rust-src` | Ch 4 | For the BPF target via `build-std` |
| unverified | aya (user space) | 0.13.x | Ch 4, 6 `hello/Cargo.toml` | crates.io current line at authoring |
| unverified | aya-ebpf (kernel) | 0.1.x | Ch 4, 6 `hello-ebpf/Cargo.toml` | crates.io current line at authoring |
| unverified | aya-log / aya-log-ebpf | 0.13.x / 0.1.x | Ch 6 | kernel log forwarding |
| unverified | aya-build | 0.1.x | Ch 6 `hello/build.rs` | build-time BPF compile; template approach may differ |
| unverified | bpf-linker | latest | Ch 4 | `cargo install`; LLVM fallback from Fedora `llvm`/`llvm-devel` |
| unverified | cargo-generate | latest | Ch 4 | scaffolds from `aya-template` |
| unverified | Podman | 5.x | Ch 1, 3 | rootless |
| unverified | podman-compose | latest | Ch 1, 3 | Fedora package |
| unverified | grafana/otel-lgtm | 0.28.0 | Ch 3 `compose.yaml` | bundles Grafana+Tempo+Mimir+Loki+Prometheus+Pyroscope+OTel Collector (+OBI); current tag at authoring |
| unverified | Python | 3.14 | Ch 3 `client/Containerfile` | UBI `ubi9/python-314` |
| unverified | opentelemetry (Rust) | 0.27.x | Ch 6 `hello/Cargo.toml` | exporter API moves between minors |
| unverified | opentelemetry (Python SDK) | 1.30.0 | Ch 3 `client/requirements.txt` | OTLP/HTTP exporter |
| unverified | libvirt / qemu-kvm / virt-install | Fedora 44 packages | Ch 1–2 | `@virtualization` group |
| unverified | bpftool / bpftrace / bcc-tools / perf | Fedora 44 packages | Ch 2, 6 | Fedora repos only (tooling policy) |

## B. Foundations — per-chapter claims (Ch 0–6, all r1.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `virt-host-validate qemu` passes hardware virtualization on a VT-x/AMD-V laptop | Ch 1 |
| unverified | Rootless `podman run ubi9/ubi-minimal echo OK` works without subscription | Ch 1 |
| unverified | `provision-vm.sh ebpf-target` boots a Fedora 44 guest via cloud-init + virt-install | Ch 2 |
| unverified | The pinned Fedora Cloud Base image filename resolves at the mirror | Ch 2 |
| unverified | The guest exposes `/sys/kernel/btf/vmlinux` (CO-RE works) | Ch 2 |
| unverified | cloud-init installs bpftool/bpftrace/bcc-tools/perf from Fedora repos in the guest | Ch 2 |
| unverified | `deploy-to-target.sh` copies a binary to the guest and runs it under sudo | Ch 2 |
| unverified | Second guest `ebpf-peer` is reachable from `ebpf-target` on the default network | Ch 2 |
| unverified | `grafana/otel-lgtm` comes up healthy under rootless Podman with the compose file | Ch 3 |
| unverified | The Python 3.14 client exports OTLP and `ebpf_events_total` appears in Grafana | Ch 3 |
| unverified | `host.containers.internal:4318` reaches the host stack from a rootless container | Ch 3 |
| unverified | `rustup` install + `1.96.0` pin + nightly `rust-src` succeed on Fedora 44 | Ch 4 |
| unverified | `cargo install bpf-linker` succeeds (or with `--no-default-features` + Fedora LLVM) | Ch 4 |
| unverified | `cargo generate` from `aya-template` produces a building workspace | Ch 4 |
| unverified | RustRover resolves the workspace's pinned toolchains per crate | Ch 4 |
| unverified | `examples/06-hello-world` builds with `cargo build --release` | Ch 6 |
| unverified | The hello tracepoint attaches to `syscalls:sys_enter_execve` and counts execve | Ch 6 |
| unverified | `hello` exports `ebpf_events_total` to the stack from the target VM | Ch 6 |
| unverified | `bpftool map dump name EVENTS` and `bpftrace` counts agree with Grafana | Ch 6 |

## C. Tracing the kernel — per-chapter claims

### Chapter 7 — kprobe + unlink (r2.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/07-kprobe-unlink` builds with `cargo build --release` | Ch 7 |
| unverified | A `#[kprobe]` attaches to `do_unlinkat` by name (entry offset 0) | Ch 7 |
| unverified | `bpf_get_current_pid_tgid`/`uid_gid`/`comm` populate the event correctly | Ch 7 |
| unverified | `do_unlinkat` 2nd arg (`struct filename *`) → `name` ptr → path string read works | Ch 7 |
| unverified | The filename read degrades gracefully (empty) on a layout mismatch, event still emitted | Ch 7 |
| unverified | `RingBuf` drains in user space via poll-on-timer; events decode via `read_unaligned` | Ch 7 |
| unverified | `ebpf_events_total{program="unlinksnoop"}` appears in Grafana | Ch 7 |
| unverified | `bpftrace -e 'kprobe:do_unlinkat { @[comm]=count() }'` counts track the tool's table | Ch 7 |

Later chapters' rows are added as each iteration drafts them (see the
[iteration roadmap](./iteration-plan.html)).

## D. Iteration log

### r1.0 — scaffold + Foundations
- **Shipped:** Jekyll site (config, layouts, includes, amber-themed CSS,
  index, Pages workflow); README, PRD, CONTRIBUTING; onboarding docs
  (README, GETTING-STARTED, LESSONS-LEARNED, STARTING-WITH-CLAUDE);
  iteration + reconciliation + prd-reconciliation plans; Chapters 0–6;
  `scripts/lab/` (provision/destroy/vm-ip/deploy + cloud-init);
  `scripts/lib/_helpers.sh` + `test-all-examples.sh`;
  `examples/03-observability-stack/` (otel-lgtm + Python 3.14 client);
  `examples/06-hello-world/` (Aya workspace + deploy).
- **Verified:** nothing — see the r1.0 honesty note above. No Fedora 44
  target was available at authoring; all code is written to current
  conventions but unrun.
- **Known risks to check first on real hardware:** (1) the Aya
  `build.rs`/`aya-build` wiring in Chapter 6 vs. whatever the current
  `aya-template` generates; (2) the exact Fedora 44 Cloud Base image
  filename in `provision-vm.sh`; (3) the `opentelemetry` 0.27 exporter
  builder names; (4) whether `aya-ebpf` now provides a panic handler,
  making ours redundant; (5) Rust `1.96.0` stable availability vs. the
  beta timing.
- **Learned:** authored against verified-current facts (Rust 1.95
  stable / 1.96 beta; aya 0.13.x; Fedora 44; otel-lgtm 0.28.0; Python
  3.14) but the lab to verify *against* is itself part of this delivery,
  so r1.1 is explicitly a verification pass.

### r2.0 — Chapter 7: kprobe + unlink
- **Shipped:** `_docs/07-kprobe-unlink.md`; `examples/07-kprobe-unlink/`
  (`unlinksnoop` workspace — kprobe on `do_unlinkat`, RingBuf events,
  OTLP reporting, `demo.sh`); reconciliation Section C rows for Ch 7.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) the filename read assumes
  `struct filename` begins with its `name` pointer — likely needs CO-RE
  field access if the layout differs; (2) `ctx.arg::<*const u8>(1)`
  argument indexing on the target kernel; (3) `RingBuf::reserve`/
  `submit` and user-side `RingBuf::next` API names in aya 0.13.x.
- **Note:** Chapter 8 (fentry + unlink) will revisit the same target to
  contrast kprobe fragility with fentry's BTF-typed argument access —
  that contrast is the pedagogical payoff, so r3.0 should be drafted to
  build directly on this chapter's code.
