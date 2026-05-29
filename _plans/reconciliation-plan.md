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

### Chapter 8 — fentry + unlink (r3.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/08-fentry-unlink` builds with `cargo build --release` | Ch 8 |
| unverified | `#[fentry]`/`#[fexit]` on `do_unlinkat` load against kernel BTF and attach | Ch 8 |
| unverified | `Btf::from_sys_fs()` + `FEntry::load("do_unlinkat", &btf)` + `attach()` API is correct | Ch 8 |
| unverified | fexit return value reads correctly as `ctx.arg::<i64>(2)` (after the 2 args) | Ch 8 |
| unverified | `HashMap<u64, UnlinkEvent>` INFLIGHT bridges entry→exit keyed by pid_tgid | Ch 8 |
| unverified | A failing unlink reports a negative `ret` (-errno); a success reports 0 | Ch 8 |
| unverified | `ebpf_events_total{program="fentrysnoop",result=...}` splits ok/fail in Grafana | Ch 8 |
| unverified | `bpftrace -e 'fexit:do_unlinkat { @[retval==0]=count() }'` split tracks the tool | Ch 8 |

### Chapters 9–10 — opensnoop, sigsnoop (r4.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/09-opensnoop` builds; tracepoints attach to `syscalls:sys_{enter,exit}_openat` | Ch 9 |
| unverified | `TracePointContext::read_at` reads filename@24/flags@32/ret@16 correctly | Ch 9 |
| unverified | `bpf_probe_read_user_str_bytes` reads the user-space filename pointer | Ch 9 |
| unverified | enter/exit pair via `HashMap` keyed by pid_tgid; result classified ok/err | Ch 9 |
| unverified | `ebpf_events_total{program="opensnoop",result=...}` appears in Grafana | Ch 9 |
| unverified | `examples/10-sigsnoop` builds; tracepoint attaches to `syscalls:sys_enter_kill` | Ch 10 |
| unverified | `sys_enter_kill` offsets pid@16/sig@24 read correctly | Ch 10 |
| unverified | signal number→name mapping + `signal` metric label work | Ch 10 |

### Chapters 11–12 — execsnoop, exitsnoop (r5.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/11-execsnoop` builds; tracepoint attaches to `sys_enter_execve` | Ch 11 |
| unverified | the bounded argv loop (array of user pointers → fixed slots) passes the verifier | Ch 11 |
| unverified | `bpf_probe_read_user` (single ptr) + `bpf_probe_read_user_str_bytes` read argv | Ch 11 |
| unverified | ~800B `ExecEvent` writes directly into the reserved ring slot (not stack) | Ch 11 |
| unverified | user space reassembles the command line; `ebpf_events_total{program="execsnoop"}` in Grafana | Ch 11 |
| unverified | `examples/12-exitsnoop` builds; tracepoint attaches to `sys_enter_exit_group` | Ch 12 |
| unverified | `error_code`@16 reads correctly; exit code decode is `& 0xff` (raw arg, not wait-encoded) | Ch 12 |
| unverified | `ebpf_events_total{program="exitsnoop",status="ok|nonzero"}` splits in Grafana | Ch 12 |

### Chapters 13–14 — bashreadline, uprobe-rust (r6.0) — *User-space & language probing*

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/13-bashreadline` builds; `#[uretprobe]` + `UProbe` attach to `readline` | Ch 13 |
| unverified | `RetProbeContext::ret()` returns the `char *`; user-string read works | Ch 13 |
| unverified | `attach(Some("readline"), 0, "/usr/bin/bash", None)` resolves the symbol on Fedora 44 | Ch 13 |
| unverified | (fallback) `readline` resolvable in `libreadline.so.8` via `READLINE_LIB` | Ch 13 |
| unverified | `examples/14-uprobe-rust` builds (snoop + target-app) | Ch 14 |
| unverified | `#[no_mangle] #[inline(never)] extern "C" compute` keeps an attachable symbol under release+LTO | Ch 14 |
| unverified | `#[uprobe]` + `ProbeContext::arg(0)` reads the C-ABI first argument | Ch 14 |
| unverified | uprobe attaches to `compute` in the deployed target-app path | Ch 14 |

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

### r3.0 — Chapter 8: fentry + unlink + CI build-on-push
- **Shipped:** `_docs/08-fentry-unlink.md`; `examples/08-fentry-unlink/`
  (`fentrysnoop` — fentry+fexit on `do_unlinkat`, `HashMap` entry→exit
  correlation, return-value capture, RingBuf events, OTLP with
  ok/fail label, `demo.sh`); reconciliation Section C rows for Ch 8;
  **`.github/workflows/pages.yml` updated** to build on every push (any
  branch) and on PRs, deploying only from `main`.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) fexit return-value access
  `ctx.arg::<i64>(2)`; (2) `Btf::from_sys_fs()` + `FEntry`/`FExit`
  `load(fn, &btf)` + `attach()` API names in aya 0.13.x; (3) the
  `struct filename` layout assumption (shared with Ch 7); (4) whether
  the target kernel permits fentry/fexit (BTF + not locked down).
- **CI note:** the deploy job is now guarded
  `if: github.event_name == 'push' && github.ref == 'refs/heads/main'`,
  so feature-branch pushes and PRs build (validate) without deploying.

### r4.0 — Chapters 9–10: opensnoop + sigsnoop
- **Shipped:** `_docs/09-opensnoop.md`, `_docs/10-sigsnoop.md`;
  `examples/09-opensnoop/` (enter+exit openat tracepoints, user-memory
  filename read, ok/err result) and `examples/10-sigsnoop/` (single
  kill tracepoint, signal name mapping); reconciliation Section C rows
  for Ch 9–10.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) tracepoint field offsets vs. the
  kernel's format files (`sys_enter_openat`, `sys_enter_kill`);
  (2) `TracePointContext::read_at::<T>(offset)` API in aya 0.13.x;
  (3) `bpf_probe_read_user_str_bytes` (user vs kernel reader) for the
  openat filename; (4) high openat event volume — future chapters add
  in-kernel filtering.
- **Teaching arc:** Ch 7 (kprobe) → Ch 8 (fentry/fexit, return values)
  → Ch 9 (stable tracepoints + user-memory reads + enter/exit) → Ch 10
  (minimal single tracepoint). The four together cover the main
  attach mechanisms before process-lifecycle tracing (execsnoop/
  exitsnoop) in r5.0.

### r5.0 — Chapters 11–12: execsnoop + exitsnoop
- **Shipped:** `_docs/11-execsnoop.md`, `_docs/12-exitsnoop.md`;
  `examples/11-execsnoop/` (execve tracepoint, bounded argv read into
  fixed slots, event written into the ring slot) and
  `examples/12-exitsnoop/` (exit_group tracepoint, exit-code decode);
  reconciliation Section C rows for Ch 11–12. Closes the "Tracing the
  kernel" part.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) the argv bounded loop passing the
  verifier — highest risk in the whole iteration set; (2)
  `bpf_probe_read_user` single-value signature; (3) writing an ~800B
  event into the ring slot via `reserve`; (4) execve/exit_group offsets;
  (5) the exit-code decode (`& 0xff` on the raw exit_group arg, NOT the
  `>> 8` used for task_struct->exit_code).
- **Correctness note:** chose the `exit_group` tracepoint over
  `sched:sched_process_exit` specifically to avoid a `task_struct`
  CO-RE read, keeping Ch 12 robust; documented that signal-deaths won't
  appear (they don't call exit_group).

### r6.0 — Chapters 13–14: bashreadline + uprobe-rust (opens User-space probing)
- **Shipped:** `_docs/13-bashreadline.md`, `_docs/14-uprobe-rust.md`;
  `examples/13-bashreadline/` (uretprobe on bash `readline`) and
  `examples/14-uprobe-rust/` (uprobe on a `#[no_mangle] extern "C"`
  function in a bundled `target-app`); reconciliation Section C rows for
  Ch 13–14. First chapters of Part "User-space & language probing".
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) `UProbe`/`#[uprobe]`/`#[uretprobe]`
  + `ProbeContext::arg`/`RetProbeContext::ret` API in aya 0.13.x;
  (2) the `attach(Some(sym), offset, target, pid)` signature;
  (3) where `readline` resolves on Fedora 44 bash (binary vs
  libreadline); (4) whether `#[inline(never)]` + `#[no_mangle]` survives
  release+LTO so `compute` stays attachable (else build target-app
  without LTO).
- **Note:** these introduce the user-space side; the new memory rule is
  "uprobe reads belong to the traced process → user probe helpers".
  Remaining Part-3 chapters (USDT, sslsniff, funclatency, runtimes) build
  on this attach model.

### r6.1 — site: two-level Part/Chapter navigation (no new chapters)
- **Shipped:** a `parts` collection (`_parts/*.md`, one per Part) +
  `_layouts/part_index.html`; the homepage now shows one **Part** card
  (with chapter count) instead of a flat chapter grid; each Part page
  lists its chapters as cards; chapter breadcrumbs link Home → Part →
  Chapter; Part pages have prev/next-part navigation. Config gains the
  `parts` collection + default `part_index` layout.
- **Rationale:** a flat 60+-chapter card grid would be unusable; the
  two-level hierarchy keeps the homepage scannable as content grows.
- **Verified:** nothing — `unverified` until the site is built. Static
  checks pass (all 10 `_parts` parse; every doc `part` matches a
  `part_name`; Liquid clean).
- **To check on build:** the `parts` collection renders at
  `/parts/<slug>/`; Part cards show correct chapter counts; empty parts
  show "Coming soon"; future chapters MUST set `part:` to the exact
  `part_name` string in the matching `_parts` file or they won't group.
