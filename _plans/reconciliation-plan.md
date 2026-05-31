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
| unverified | Python | 3.14 | Ch 3 `client/Containerfile` | UBI `ubi9/python-314`; clients + FastAPI target |
| unverified | FastAPI | current | Ch 16 (Python target) | Python 3.14 app target, containerized |
| unverified | Java | 25 (LTS) | Ch 16 (Java target) | Quarkus runtime |
| unverified | Quarkus | 3.33 (LTS) | Ch 16 (Java target) | containerized, UBI + multi-stage |
| unverified | crun | 1.27.1 | Ch 16, container-observation chapters | Fedora default OCI runtime; eBPF + SELinux |
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

### Chapter 15 — BTF-assisted uprobe (r7.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/15-btf-uprobe` builds (snoop + target-app with debug info) | Ch 15 |
| unverified | `bpf_probe_read_user::<Order>(ptr)` copies a whole struct from target memory | Ch 15 |
| unverified | `ProbeContext::arg(0)` yields the `*const Order` pointer | Ch 15 |
| unverified | shared `#[repr(C)] Order` layout matches across app/ebpf/user | Ch 15 |
| unverified | `pahole -J` + `bpftool btf dump file target-app` shows `Order` with offsets | Ch 15 |
| unverified | `ebpf_events_total{program="btf-uprobe",status=...}` appears in Grafana | Ch 15 |

### Chapter 16 — containerized targets + cgroup-scoped observation (r8.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | FastAPI target builds (multi-stage `ubi9/python-314` → `-minimal`) and serves `/work` | Ch 16 |
| unverified | Quarkus target builds (UBI OpenJDK 25 + Quarkus 3.33 fast-jar) and serves `/work` | Ch 16 |
| unverified | `ubi9/openjdk-25` image tag exists (else use Containerfile fallback) | Ch 16 |
| unverified | target VM has podman/crun/dwarves after re-provision (added to cloud-init) | Ch 16 |
| unverified | `examples/16-container-targets` (contrace) builds | Ch 16 |
| unverified | `bpf_get_current_cgroup_id()` + `Array::set` cgroup filter works in aya 0.13.x | Ch 16 |
| unverified | container cgroup id resolves via `podman inspect CgroupPath` + `stat -c %i` | Ch 16 |
| unverified | scoped contrace emits only the target container's opens; PID is host PID | Ch 16 |
| unverified | `ebpf_events_total{program="contrace",container=...}` per-container series in Grafana | Ch 16 |

### Chapters 17–18 — sslsniff, funclatency (r9.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/17-sslsniff` builds; 3 uprobes attach to `libssl.so.3` (SSL_write, SSL_read enter+ret) | Ch 17 |
| unverified | `SSL_write` buf-at-entry and `SSL_read` buf-at-return (stashed) capture plaintext | Ch 17 |
| unverified | `bpf_probe_read_user_buf` with a min(len,CAP) dynamic length passes the verifier | Ch 17 |
| unverified | OpenSSL 3 `SSL_read`/`SSL_write` symbol names resolve on Fedora 44 | Ch 17 |
| unverified | `ebpf_events_total{program="sslsniff",dir=...}` splits read/write in Grafana | Ch 17 |
| unverified | `examples/18-funclatency` builds (snoop + target-app) | Ch 18 |
| unverified | uprobe-entry `bpf_ktime_get_ns` stash + uretprobe delta via `START` HashMap works | Ch 18 |
| unverified | `slow_op` stays attachable under release+LTO (`#[inline(never)]`) | Ch 18 |
| unverified | per-call `delta_ns` records into OTLP `f64_histogram` `function_latency_ms` | Ch 18 |
| unverified | console log2 ASCII histogram renders | Ch 18 |

### Chapters 19–20 — goroutine states, javagc (r10.0) — *closes User-space & language probing*

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/19-goroutine-states` builds (Go target via `go build` + tracer) | Ch 19 |
| unverified | uprobe on `runtime.casgstatus`; `newval` read from RCX via `pt_regs` (Go ABI) | Ch 19 |
| unverified | `pt_regs.rcx` field name correct in aya 0.13.x bindings | Ch 19 |
| unverified | NO uretprobe used on Go (uprobe only) — documented hazard | Ch 19 |
| unverified | goroutine-state value→name mapping correct for the Go version | Ch 19 |
| unverified | `examples/20-javagc` builds | Ch 20 |
| unverified | USDT `gc__begin`/`gc__end` offsets resolve from `readelf -n` stapsdt notes | Ch 20 |
| unverified | uprobe attach by offset (`attach(None, off, libjvm, None)`) hits the USDT site | Ch 20 |
| unverified | readelf Location == uprobe file offset (else vaddr→offset conversion needed) | Ch 20 |
| unverified | begin/end timing via `GC_START` HashMap; `jvm_gc_pause_ms` OTLP histogram | Ch 20 |
| unverified | JDK ships hotspot USDT probes; `-XX:+ExtendedDTraceProbes` enables them | Ch 20 |

### Chapters 21–22 — runqlat, hardirqs (r11.0) — *opens Performance & resources*

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/21-runqlat` builds | Ch 21 |
| unverified | sched tracepoint offsets correct (prev_pid@24, prev_state@32, next_pid@56, wakeup pid@24) | Ch 21 |
| unverified | `TASK_RUNNING == 0` for the preempted-task re-stamp | Ch 21 |
| unverified | in-kernel log2 histogram via `Array::get_ptr_mut` increments correctly | Ch 21 |
| unverified | OTLP observable gauge `runqueue_latency_us{quantile}` (registered-once callback) works in otel 0.27 | Ch 21 |
| unverified | `examples/22-hardirqs` builds | Ch 22 |
| unverified | `irq` field offset (@8) in irq_handler_entry/exit format | Ch 22 |
| unverified | per-CPU keying via `bpf_get_smp_processor_id`; nested-IRQ simplification acceptable | Ch 22 |
| unverified | per-IRQ `HashMap<u32, IrqStat>` accumulation; user-space `iter()` read | Ch 22 |
| unverified | OTLP observable gauge `hardirq_total_ns{irq}` works in otel 0.27 | Ch 22 |

### Chapter 23 — profile (r12.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/23-profile` builds | Ch 23 |
| unverified | `PerfEvent::attach(PerfTypeId::Software, 0, AllProcessesOneCpu, Frequency(99))` signature in aya 0.13.x | Ch 23 |
| unverified | `online_cpus()` return/error type as used | Ch 23 |
| unverified | `StackTrace::get_stackid(&ctx, flags)` (ebpf) for kernel (0) + user (BPF_F_USER_STACK) | Ch 23 |
| unverified | `StackTraceMap::get(&id,0).frames()` + `frame.ip` (user) | Ch 23 |
| unverified | `aya::util::kernel_symbols()` BTreeMap for kernel symbolization | Ch 23 |
| unverified | user-stack capture works for target (frame pointers / unwind) | Ch 23 |
| unverified | folded output pipes to flamegraph.pl; Pyroscope push left as extension | Ch 23 |

### Chapters 24–25 — memleak, biopattern (r13.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/24-memleak` builds (+ leaker.c compiles on VM with clang) | Ch 24 |
| unverified | uprobe+uretprobe on glibc `malloc`/`calloc` + uprobe on `free` attach by symbol | Ch 24 |
| unverified | malloc size@entry / ptr@return bridged by `SIZES` HashMap; `ALLOCS[ptr]` add/remove | Ch 24 |
| unverified | `bpf_get_stackid(BPF_F_USER_STACK)` captures alloc site; user-stack needs frame pointers | Ch 24 |
| unverified | `TARGET_PID` `Array` pid filter; `u64_gauge` in otel 0.27 | Ch 24 |
| unverified | `examples/25-biopattern` builds | Ch 25 |
| unverified | `block_rq_issue` field offsets (dev@8, sector@16, nr_sector@24) match format file | Ch 25 |
| unverified | per-device `LAST_END`/`STATS` HashMaps; sequential = (sector == last_end) | Ch 25 |
| unverified | `dev_t` major:minor decoding correct | Ch 25 |
| unverified | OTLP observable gauge `bio_sequential_ratio{dev}` in otel 0.27 | Ch 25 |

### Chapter 26 — energy monitoring (r14.0) — *closes Performance & resources*

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/26-energy` builds | Ch 26 |
| unverified | `sched_switch` offsets (prev_comm@8, prev_pid@24) read correctly | Ch 26 |
| unverified | per-task `cpu_ns` credited on switch-out via `ONCPU`/`USAGE` HashMaps | Ch 26 |
| unverified | RAPL `/sys/class/powercap/.../energy_uj` read; absent on VM → flat TDP model | Ch 26 |
| unverified | per-comm power = share × system_w; observable gauges `estimated_power_watts{comm}` + `system_power_watts` in otel 0.27 | Ch 26 |
| note | accuracy upgrade (PERF_EVENT_ARRAY + bpf_perf_event_read_value cycles, needs vPMU) left as documented extension | Ch 26 |

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

### r6.2 — docs: neutral source-provenance wording
- **Shipped:** removed country-specific naming from the source-provenance
  policy across CONTRIBUTING.md, PRD.md, README.md, _docs/00-outline.md,
  _plans/prd-reconciliation.md, onboarding/STARTING-WITH-CLAUDE.md.
  Replaced with a neutral norm: insight from anywhere; ship original,
  clearly-licensed code; no line-for-line copying/porting.
- **Verified:** N/A (wording change). No country references remain in
  the repo.

### r7.0 — Chapter 15: BTF-assisted uprobe
- **Shipped:** `_docs/15-btf-uprobe.md`; `examples/15-btf-uprobe/`
  (target-app passing a `*const Order`, a uprobe reading the whole
  struct via `bpf_probe_read_user`, BTF inspection via `pahole -J` +
  `bpftool btf dump`); reconciliation Section C rows for Ch 15.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) `bpf_probe_read_user::<T>` reading
  a whole struct (vs str/bytes variants) in aya 0.13.x; (2)
  attachability of `process_order` under release+LTO; (3) `debug = true`
  leaving DWARF that `pahole -J` can convert; (4) `Order` `#[repr(C)]`
  offsets matching the BTF dump.
- **Scope note:** framed user-space CO-RE honestly — kernel CO-RE is
  turnkey, user-space relocation is newer; the robust path taught is the
  shared/BTF-generated `#[repr(C)]` mirror + `bpf_probe_read_user`. Full
  relocation deferred to the CO-RE deep-dive (Ch 56).

### r7.1 — conventions: container policy, version pins, first diagram
- **Shipped:** expanded CONTRIBUTING container policy (everything
  user-space in Podman except the privileged Aya loader on the VM;
  **multi-stage UBI** Containerfiles mandatory; crun 1.27.1 + container
  observation + SELinux; Java 25/Quarkus 3.33 + Python 3.14/FastAPI
  target pins; Excalidraw diagram workflow). PRD + reconciliation
  version table updated with Java/Quarkus/FastAPI/crun. Rewrote the
  Ch 3 client Containerfile as multi-stage UBI (builder venv → minimal
  runtime). Added the first real diagram — `assets/diagrams/lab-topology`
  (.svg + .excalidraw) — embedded in Chapter 2.
- **Verified:** N/A for policy/diagram; the multi-stage Containerfile is
  `unverified` (not built here).
- **To check on build:** the multi-stage build resolves
  `ubi9/python-314` (builder) + `ubi9/python-314-minimal` (runtime) and
  the venv copy runs; the SVG renders via the excalidraw include at
  `/assets/diagrams/lab-topology.svg`.
- **Captured as durable project requirements** (carried forward to all
  future chapters): the container/loader split, multi-stage UBI, the
  language-target pins, crun coverage, and the Excalidraw workflow.

### r8.0 — Chapter 16: containerized targets + cgroup-scoped observation
- **Shipped:** `_docs/16-container-targets.md`;
  `examples/16-container-targets/` — FastAPI (Python 3.14) and Quarkus
  (Java 25 + Quarkus 3.33) targets as multi-stage UBI containers,
  `contrace` (cgroup-scoped openat tracer via
  `bpf_get_current_cgroup_id()` + config `Array`), `compose.yaml`, and a
  demo that runs targets on the VM and scopes observation to one
  container. Added `podman`/`crun`/`dwarves` to the target VM cloud-init.
  Reconciliation Section C rows for Ch 16.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) cgroup-id resolution
  (rootless/rootful + cgroup-manager variance) — demo falls back to
  unscoped; (2) `ubi9/openjdk-25` tag availability + Quarkus 3.33 build
  (Containerfile carries a fallback); (3) `bpf_get_current_cgroup_id` /
  `Array::set` API; (4) re-provisioning the VM so podman/crun are
  present.
- **Model refinement:** observed app-target containers run **on the
  target VM** (not the host), because eBPF attaches to that kernel —
  documented in the chapter and the lab topology applies. Host still
  runs the stack + load driver.

### r9.0 — Chapters 17–18: sslsniff + funclatency
- **Shipped:** `_docs/17-sslsniff.md`, `_docs/18-funclatency.md`;
  `examples/17-sslsniff/` (uprobes on libssl SSL_write/SSL_read,
  entry/return correlation for reads, bounded plaintext capture, ethics
  note) and `examples/18-funclatency/` (uprobe+uretprobe timing with
  `bpf_ktime_get_ns`, per-call ring events → OTLP `f64_histogram`,
  console log2 histogram, bundled target-app). Reconciliation Section C
  rows for Ch 17–18.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) `bpf_probe_read_user_buf` with a
  dynamic `min(len,CAP)` length passing the verifier (sslsniff); (2)
  OpenSSL 3 symbol names in libssl.so.3; (3) `bpf_ktime_get_ns` +
  uprobe/uretprobe-on-same-symbol + entry/exit HashMap; (4)
  `f64_histogram` in opentelemetry 0.27; (5) `slow_op`/SSL symbols
  attachable under release+LTO.
- **Note:** funclatency ships the per-call-event approach (simple, OTLP
  histogram); the in-kernel-histogram optimization is documented as the
  high-call-rate alternative. This closes the core techniques of Part
  "User-space & language probing"; remaining Part-3 chapters (goroutine
  states, javagc) apply them to specific runtimes.

### r10.0 — Chapters 19–20: goroutine states + javagc (closes Part 3)
- **Shipped:** `_docs/19-goroutine-states.md`, `_docs/20-javagc.md`;
  `examples/19-goroutine-states/` (Go target + uprobe on
  `runtime.casgstatus` reading the Go-ABI RCX register, uprobe-only) and
  `examples/20-javagc/` (HotSpot USDT `gc__begin`/`gc__end` timed via
  uprobes at resolved offsets, OTLP GC-pause histogram, Java target).
  Reconciliation Section C rows for Ch 19–20.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) Go register ABI read (RCX) +
  `pt_regs.rcx` field name; (2) USDT offset resolution and the
  readelf-Location→file-offset assumption (javagc — most experimental);
  (3) Go symbol presence; (4) JDK hotspot USDT availability; (5) new host
  toolchains needed: `golang` (Ch 19) and a JDK (Ch 20), both Fedora
  repos.
- **Two hazards documented as first-class lessons:** never uretprobe Go
  (moving stacks corrupt return trampolines); non-C languages need
  manual register mapping (ctx.arg assumes the C ABI). bpftrace's native
  USDT + `reg()` are the cross-check references.
- **Milestone:** Parts 0–3 complete (Foundations, Tracing the kernel,
  User-space & language probing) — 21 chapters, 16 examples. Next:
  Part 4 Performance & resources (r11+).

### r11.0 — Chapters 21–22: runqlat + hardirqs (opens Part 4)
- **Shipped:** `_docs/21-runqlat.md`, `_docs/22-hardirqs.md`;
  `examples/21-runqlat/` (sched_wakeup/_new + sched_switch, in-kernel
  log2 histogram in an `Array`, OTLP observable-gauge percentiles) and
  `examples/22-hardirqs/` (irq_handler_entry/exit keyed by CPU, per-IRQ
  `HashMap` totals, OTLP observable-gauge per vector). Reconciliation
  Section C rows for Ch 21–22.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) sched + irq tracepoint field
  offsets; (2) `Array::get_ptr_mut` / `HashMap` accumulation in-kernel;
  (3) the **observable-gauge callback** API in opentelemetry 0.27 (first
  use of registered-once observable gauges — both chapters); (4)
  `TASK_RUNNING==0` and nested-IRQ simplification.
- **Technique milestone:** both chapters use **in-kernel aggregation**
  (the hot-path technique flagged in Ch 18) rather than per-event ring
  buffers — runqlat via a log2 `Array` histogram, hardirqs via a per-IRQ
  `HashMap`. This is the Performance-part idiom.

### r12.0 — Chapter 23: profile (sampling CPU profiler)
- **Shipped:** `_docs/23-profile.md`; `examples/23-profile/` — a
  `perf_event` program sampling at 99 Hz on every CPU, capturing kernel
  + user stacks via `bpf_get_stackid` into a `StackTrace` map, a count
  map keyed by `(pid,comm,kstack,ustack)`, kernel symbolization via
  `kernel_symbols()`, and **folded** flame-graph output. Reconciliation
  Section C rows for Ch 23.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) the `PerfEvent::attach` signature
  / `SamplePolicy` / `PerfEventScope` and `online_cpus()` in aya 0.13.x
  (first `perf_event` program); (2) `get_stackid` (ebpf) and
  `StackTraceMap::get().frames()` (user) — first stack-walking example;
  (3) user-stack capture depending on frame pointers/unwind info.
- **New ground:** first `perf_event` program type, first stack walking,
  first sampling (fixed-cost) tool. User-frame symbolization left as hex
  (wire in `blazesym`); Pyroscope push noted as the continuous-profiling
  extension (otel-lgtm bundles Pyroscope).

### r13.0 — Chapters 24–25: memleak + biopattern
- **Shipped:** `_docs/24-memleak.md`, `_docs/25-biopattern.md`;
  `examples/24-memleak/` (malloc/calloc+free pairing, alloc-site stacks
  via `bpf_get_stackid` reused from Ch 23, pid-scoped, bundled leaker.c)
  and `examples/25-biopattern/` (block_rq_issue tracepoint, per-device
  sequential/random classification by sector arithmetic, OTLP ratio
  gauge). Reconciliation Section C rows for Ch 24–25.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) glibc malloc/calloc/free uprobe
  attach + user-stack capture needing frame pointers (memleak); (2)
  `block_rq_issue` field offsets vs. the format file (biopattern —
  layout has drifted across kernels); (3) `dev_t` decode; (4) `u64_gauge`
  and observable-gauge APIs in otel 0.27.
- **Continuity:** memleak deliberately reuses the Ch 23 stack-walking
  primitive (StackTrace + get_stackid), reinforcing it as a building
  block. Both remain consistent with the aggregate-in-kernel idiom.

### r14.0 — Chapter 26: energy monitoring (closes Part 4)
- **Shipped:** `_docs/26-energy.md`; `examples/26-energy/` — a
  `sched_switch` tracepoint crediting per-task on-CPU time
  (`ONCPU`/`USAGE` HashMaps), with user-space energy attribution by
  CPU-time share × system power (RAPL when present, flat-TDP model when
  not — the VM reality). Exports `estimated_power_watts{comm}` +
  `system_power_watts`. Reconciliation Section C rows for Ch 26.
- **Verified:** nothing — `unverified` pending a real Fedora 44 run.
- **Known risks to check first:** (1) sched_switch offsets; (2) RAPL
  absence on KVM guests (fallback model exercised); (3) observable-gauge
  API in otel 0.27; (4) the model is an estimate by construction.
- **Honesty note:** chapter is explicit that RAPL/vPMU are usually NOT
  exposed in VMs, so absolute watts are modeled on the lab VM while the
  attribution (shares) stays correct; bare metal gives real RAPL. This
  mirrors Kepler's cloud accommodation. Hardware-counter accuracy upgrade
  (PERF_EVENT_ARRAY + bpf_perf_event_read_value) documented as the
  extension, not shipped, to avoid an uncertain API in the core example.
- **MILESTONE: Part 4 (Performance & resources) complete** — Ch 21–26
  (runqlat, hardirqs, profile, memleak, biopattern, energy), 27 chapters
  / 22 examples total. Parts 0–4 done. Next: Part 5 Networking (r15+),
  which needs the two-VM peer build-out.

### r14.1 — diagrams pass (Tier 1–3)
- **Shipped:** 19 new Excalidraw+SVG diagram pairs under
  `assets/diagrams/` (plus a spec-based `generate.py`), embedded one or
  two per chapter across Ch 3,4,5,6,8,9,11,13,15,16,17,19(×2),20,21,23,
  24,25,26. Covers the foundational/reusable concepts (lifecycle,
  RingBuf path, data path, workspace build, entry/exit correlation,
  user-vs-kernel memory reads), the language-probing mechanics
  (probing-surfaces menu, struct/BTF, container observation, TLS
  boundary, goroutine state machine, Go-vs-C ABI, USDT-as-uprobe), and
  the performance pipelines (runqlat timeline, profiler, memleak, bio
  seq/random, energy attribution). README catalogue updated.
- **Verified:** SVGs are well-formed XML; Excalidraw files parse; all
  chapter front matter still parses; includes use the existing
  `excalidraw.html` partial with alt text + figure captions.
- **Not verified (rendering):** exact visual layout/overflow in a real
  Jekyll build (no local Jekyll); `.svg` are clean themed exports rather
  than the hand-drawn Excalidraw aesthetic — `.excalidraw` sources are
  included so they can be refined/re-exported.
- **Deferred by design:** networking diagrams (packet path / hook
  points, two-VM topology, TCP lifecycle, XDP-vs-tc) ship with the
  Part 5 chapters (r15+).

### Chapters 27–28 — tcpconnlat, tcpstates (r15.0) — *opens Networking; two-VM*

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | two-VM build-out: `provision-vm.sh ebpf-peer`; both guests reach each other on the libvirt NAT net | Ch 27 |
| unverified | cloud-init adds nmap-ncat/socat/iproute/iputils/curl/tcpdump to guests | Ch 27 |
| unverified | `examples/27-tcpconnlat` builds | Ch 27 |
| unverified | kprobe `tcp_v4_connect` + `tcp_rcv_state_process` attach; keyed by struct sock* | Ch 27 |
| unverified | `sock_common` offsets skc_daddr@0 / skc_dport@12 read correctly (CO-RE in Ch 56) | Ch 27 |
| unverified | first-`tcp_rcv_state_process`≈SYN-ACK assumption holds for active connects | Ch 27 |
| unverified | `tcp_connect_latency_ms` histogram in Grafana | Ch 27 |
| unverified | `examples/28-tcpstates` builds | Ch 28 |
| unverified | `sock:inet_sock_set_state` tracepoint offsets (oldstate@16/newstate@20/sport@24/dport@26/protocol@30/saddr@32/daddr@36) | Ch 28 |
| unverified | sport/dport byte order as stored by the tracepoint | Ch 28 |
| unverified | `ebpf_tcp_state_transitions_total{newstate}` in Grafana | Ch 28 |

### r15.0 — Chapters 27–28: tcpconnlat + tcpstates (opens Part 5, two-VM)
- **Shipped:** `_docs/27-tcpconnlat.md`, `_docs/28-tcpstates.md`;
  `examples/27-tcpconnlat/` (kprobes on the TCP connect path, sock* key,
  sock-field offset reads) and `examples/28-tcpstates/`
  (sock:inet_sock_set_state tracepoint, full state machine). Two-VM
  build-out: cloud-init net tools (nmap-ncat/socat/iproute/iputils/curl/
  tcpdump), `scripts/lab/lab-ips.sh` helper; peer is
  `provision-vm.sh ebpf-peer`. Networking diagrams authored:
  `net-hooks`, `tcp-handshake` (Ch 27), `tcp-states` (Ch 28).
- **Verified:** nothing — `unverified` pending real Fedora 44 + two VMs.
- **Known risks to check first:** (1) `sock_common` offsets (Ch 27 —
  CO-RE removes this in Ch 56); (2) inet_sock_set_state tracepoint
  offsets + port byte order (Ch 28); (3) kprobe attach to TCP symbols;
  (4) two guests routing to each other.
- **Pedagogical contrast (deliberate):** Ch 27 kprobe + struct-offset
  (powerful, fragile) vs. Ch 28 stable tracepoint (less reach, durable)
  — stated explicitly as the kernel-tracing through-line.
- **Networking diagrams shipped with the chapters** (per the r14.1 plan
  to fold net diagrams into r15+).

### Chapters 29–30 — HTTP L7, sockops (r16.0)

| Status | Claim | Chapter |
|--------|-------|---------|
| unverified | `examples/29-http-l7` builds | Ch 29 |
| unverified | `socket_filter` program + `SkBuffContext` load/load_bytes in aya 0.13.x | Ch 29 |
| unverified | AF_PACKET raw socket setup (libc) + `SocketFilter::attach(fd)` | Ch 29 |
| unverified | Eth→IPv4(no-options)→TCP parse + data-offset math reaches payload | Ch 29 |
| unverified | HTTP method/`HTTP/` detection + first-line capture; cleartext only | Ch 29 |
| unverified | `ebpf_http_lines_total{method}` in Grafana | Ch 29 |
| unverified | `examples/30-sockops` builds | Ch 30 |
| unverified | `sock_ops` program + `SockOps::attach(cgroup)` in aya 0.13.x | Ch 30 |
| unverified | `SockOpsContext` accessors (op/local_ip4/remote_ip4/local_port/remote_port) | Ch 30 |
| unverified | established op constants (ACTIVE=4/PASSIVE=5); port byte-order convention | Ch 30 |
| unverified | requires cgroup-v2 at /sys/fs/cgroup; `ebpf_sock_established_total{dir}` in Grafana | Ch 30 |

### r16.0 — Chapters 29–30: HTTP L7 + sockops
- **Shipped:** `_docs/29-http-l7.md`, `_docs/30-sockops.md`;
  `examples/29-http-l7/` (socket_filter parsing Eth/IPv4/TCP → HTTP line,
  AF_PACKET raw socket) and `examples/30-sockops/` (sock_ops on the
  cgroup-v2 root, established callbacks, 4-tuple from the context).
  Diagrams `l7-socketfilter` (Ch 29) and `sockops-cb` (Ch 30).
- **Verified:** nothing — `unverified` pending real Fedora 44 + two VMs.
- **Known risks to check first:** (1) socket_filter + SkBuffContext API
  and AF_PACKET setup (Ch 29 — first packet-content program); (2)
  sock_ops attach + SockOpsContext accessors + op constants (Ch 30 —
  first cgroup-attached callback program); (3) byte-order conventions;
  (4) IHL==5 / cleartext-only simplifications.
- **Two new program types** introduced (socket_filter, sock_ops). L7
  taught both ways: socket filter (wire/cleartext) vs. syscall+uprobe
  (buffer/encrypted, ties back to Ch 17). sock_ops framed as the
  observe-and-act, cgroup-scoped, callback model.

### r16.1 — quality pass, part 1 (diagrams, VM/networking setup, code-depth model)
- **#3 diagrams (fixed):** generator now draws nodes before edges so
  arrowheads render *on top* of boxes (root cause of "arrows go behind
  boxes / stop short"); arrowheads enlarged; edge labels get a white halo.
  All 24 edge-bearing SVGs reordered; `goroutine-states` redesigned with
  short adjacent arrows instead of one long cross-diagram arrow.
  `assets/diagrams/generate.py` synced to the fixed generator.
- **#4 VM/networking setup (clarified):** the two-VM lab already lives in
  Foundations (Ch 2); added an explicit "What the networking part needs"
  subsection (peer reachability / interface / cgroup-v2 table + resource
  sizing) and a back-reference from Ch 27 to Ch 2.
- **#2 code-explanation depth (model established):** Ch 9 (opensnoop)
  rewritten with a full "How the code works" walkthrough — both maps and
  why each type, the entry stash and exit pair/emit with per-call
  explanation, and the user-space attach + ring-drain + decode. This is
  the depth/style to roll through the remaining chapters in subsequent
  passes (pending author confirmation).
- **Verified:** nothing new — diagrams are visual-only; code remains
  `unverified`.

### r16.2 — code-depth pass, Part 1 (Tracing the kernel)
- Brought Part 1 to the Ch 9 walkthrough standard ("explain what Rust+Aya
  are doing, as if the reader were writing it, with a BCC side-by-side").
- **Deepened:** Ch 10 (sigsnoop) — full kernel handler explained + the
  previously-absent user-side attach/drain/format walkthrough; Ch 12
  (exitsnoop) — real handler instead of a stub + concrete user-side
  decode (kept the strong "encoding gotcha" section).
- **Already at standard (left as-is):** Ch 7 (kprobe), Ch 8 (fentry —
  BTF load explained), Ch 11 (execsnoop — argv-loop walkthrough).
- Docs-only; code remains unverified.

### r16.3 — code-depth pass, Part 2 (User-space & language probing)
- Reviewed Ch 13–20 against the Ch 9 standard. **Already at depth (left
  as-is):** Ch 14 (Rust uprobe / no_mangle), Ch 15 (struct read + BTF),
  Ch 16 (cgroup-id scoping handler + inode resolution), Ch 17 (sslsniff
  entry/return library probes), Ch 18 (entry/return timing + in-kernel-
  vs-userspace histogram trade-off), Ch 19 (Go ABI pt_regs/RCX read,
  no-uretprobe-on-Go), Ch 20 (USDT-as-uprobe at offset). These already
  show real handlers, the Aya mechanics, and register/BTF cross-checks.
- **Deepened:** Ch 13 (bashreadline) — replaced the stub uretprobe body
  with the full reserve/read-user-string/submit handler + explanation.
- Docs-only; code remains unverified.

### r16.4 — diagram + wording fixes; Part 3 review
- **Diagram fix:** `container-observe` (Ch 16) rebuilt — the "container"
  is now a framing **band** (top-left label) so no centered text sits
  behind the nested app/libjvm boxes. Audited all diagrams for
  node-contains-node label-hiding; only this one was affected
  (`lab-topology`'s labels are top-left, so it was a false positive).
- **Wording:** removed all "roadmap" closers and the "(iteration)
  roadmap" links from every chapter; removed "honest"/"an honest"
  framing (6 spots) — reworded, e.g. Ch 15 "An honest scope note" → "Scope
  note", Ch 26 "The honest part" → "The hard part". Repaired the two
  sentences left dangling by the removals (Ch 0, Ch 7).
- **Nav:** dropped the public "Roadmap" menu item; the "Tutorial" item
  already points to the outline (more useful to readers).
- **Ch 17 (sslsniff):** added a **FIPS** section — FIPS mode changes the
  cipher/provider, not where plaintext sits, so the SSL_read/SSL_write
  uprobes capture identically; with the kTLS caveat as the real
  boundary-mover.
- **Part 3 (Ch 21–26) review:** already at the Ch 9 depth standard
  (in-kernel `Array` log2 histogram + `get_ptr_mut`; per-key/per-CPU
  HashMap aggregation with race + frame-pointer notes; `get_stackid`
  stack walking; OTLP **observable-gauge** percentile callbacks; RAPL +
  VM fallback). No rewrites needed.

### r16.5 — code-depth pass, Part 4 (Networking) — rollout complete
- Brought Ch 27–30 to the Ch 9 walkthrough standard (these were the
  tersest chapters). Each now shows the real handler(s) and a user-side
  walkthrough with per-call explanation:
  - Ch 27 (tcpconnlat): full two-kprobe handlers (sock* key, struct-field
    reads), user-side dual `KProbe` attach + ring drain + OTLP histogram.
  - Ch 28 (tcpstates): full tracepoint handler (PROTOCOL filter, [u8;4]
    addr reads), user-side attach + state-name map + counter.
  - Ch 29 (http-l7): completed the parse→capture→submit, plus the
    distinctive AF_PACKET raw-socket open + `SocketFilter::attach(fd)`.
  - Ch 30 (sockops): concrete `ctx.op()` handler with reserve/submit,
    plus cgroup-v2 `SockOps::attach(File)` user side.
- **Code-depth rollout now complete** across Parts 0–4: Part 1 (r16.2)
  deepened Ch 10/12; Part 2 (r16.3) deepened Ch 13; Parts 2/3 otherwise
  already at depth; Part 4 (r16.5) deepened all four. Docs-only; code
  remains unverified.

### r16.6 — diagram 27.2 fix + Ch 20 (JVM/GC) expansion
- **Diagram fix:** `tcp-handshake` (Fig 27.2) redrawn — both kprobe boxes
  now sit above the client and each dashed connector lands on its event
  (connect→client, rcv_state_process→the SYN-ACK arrow); the previous
  dangling diagonal is gone.
- **Ch 20 (javagc) expanded** per request:
  - New `jvm-observable` diagram (Excalidraw+SVG): the HotSpot USDT probe
    surface across GC / memory pools / JIT / threads / monitors /
    allocation+classes, each reachable as a uprobe at its offset.
  - Collectors: noted the JDK/OpenJDK-UBI ships G1 (default), ZGC, and
    Shenandoah; all fire gc__begin/gc__end, but ZGC/Shenandoah are mostly
    concurrent so the signal shifts to concurrent-cycle/allocation.
  - "Many JVMs on one node": one probe per distinct libjvm.so path
    (per container overlay), attribute by cgroup/PID, label by
    container+collector (ties to Ch 16).
  - "Why GC monitoring matters": stop-the-world pauses as tail latency,
    time-in-GC as saturation, what to alert on, and the out-of-process
    advantage (no JVM flags / agent / verbose:gc parsing).
- Diagram catalogue updated (25 → 26 diagrams). Docs/diagrams only.

### r16.7 — runnability + part-numbering pass (pre-r17)
- **Part numbering fixed:** homepage cards render `Part {order}`, so
  Networking (order 4) is **Part 4**. Corrected stale "Part 5" refs in
  Ch 26 and Ch 27 (description + prose). ("Part 6" in Ch 26 = sched_ext,
  order 6 — correct, left.)
- **Ch 27 environment readiness:** added an up-front "Before you start —
  this part needs two VMs" block (stack up / target running / peer
  provisioned, with the exact checks) so the reader confirms the
  environment before running; trimmed the now-redundant mid-chapter
  peer-provisioning into a short topology note.
- **Explicit run guidance in every chapter:** appended a one-line run
  hint to each program chapter's code-location line ("`./demo.sh` there
  builds, deploys, and runs it; its `README.md` covers …") — 25 chapters
  (06–26, 28–30) plus tailored hints for Ch 03/14/20; Ch 27 carries the
  fuller block.
- **Getting-Started:** added a reader-facing "Running any chapter's
  example" section (the universal `cd examples/NN` → `./demo.sh`
  pattern, README per example, demo.sh self-docs, and the
  stack/target/peer assumptions).
- Docs-only. No test runner exists (`test-all-examples.sh` was stale in
  notes); examples are exercised via each `demo.sh` + `cargo build`.

### r17.0 — Part 4 (Networking) continues: tc + first XDP — UNVERIFIED
New chapters and examples (all unverified — not yet run on Fedora 44):
- **Ch 31 (tc-classify)** — `#[classifier]` on clsact egress; counts
  packets/bytes per L4 proto in-kernel HashMaps; drops traffic to
  BLOCK_PORT with `TC_ACT_SHOT`. New diagram `tc-clsact`. Risks to
  confirm: Aya tc API (`qdisc_add_clsact`, `SchedClassifier`,
  `TcAttachType::Egress`), `network-types` 0.0.7 field/LEN names,
  `TcContext::load`/`len`, user-space `HashMap::iter()/get` deltas,
  that `TC_ACT_SHOT` drops on egress.
- **Ch 32 (xdp-drop)** — `#[xdp]` ingress; raw `data`/`data_end` with a
  `ptr_at` bounds check; counts per proto, drops ICMP with `XDP_DROP`.
  New diagram `xdp-path`. Risks: `virtio-net` native XDP vs the
  `SKB_MODE` fallback, `XdpContext::data/data_end`, verifier acceptance
  of `ptr_at`, that `ping` to the target stops while attached.
- Both reinforce the in-kernel-aggregation lesson (no per-packet ring on
  a data path) and introduce **verdicts** (acting, not just observing).
- New shared dep introduced: `network-types = "0.0.7"` (header parsing).

### r17.1 — diagram fix + command-block clarity (docs only)
- Fixed Fig 31.1 (`tc-clsact`): the two hook boxes were covering the
  band's top-left "clsact qdisc (eth0)" label; boxes now start well below
  the label so it's legible.
- Added a "Reading the command blocks" callout to Ch 2 defining the
  prompt prefixes (`[host]$`/`[vm]$`/`[peer]$`) and the `<…>` placeholder
  convention, with how to get a shell and find `<iface>`.
- Standardized the new networking chapters on `[vm]$`/`[peer]$` (were
  `[target]$`); rewrote the Ch 31 and Ch 32 cross-checks to discover the
  interface first (`ip -br link`, concrete `enp1s0`), show option meanings
  (`tc -s`, `ip link show`, `bpftool net show`) and expected output, and
  define `<target-ip>` with a concrete example. No remaining bare
  command-block placeholders.

### r17.2 — document the lab credential model (docs only)
- Made the lab credential model explicit in the Ch 2 conventions callout:
  user `fedora`, SSH-key-only login (account password locked via cloud-init
  `lock_passwd: true`), and passwordless sudo (`NOPASSWD:ALL`) — so there is
  no sudo password to list, and every `[vm]$ sudo …` runs without a prompt.
  Noted how to set a console password if ever needed.

### r18.0 — Part 4 (Networking): XDP capture + XDP load balancer — UNVERIFIED
- **Ch 33 (xdp-capture)** — `#[xdp]` read-only tap: parse Eth/IPv4/TCP via
  `ptr_at`, count all packets in `SEEN`, and ship a `FlowRecord` over a
  `RingBuf` only for TCP SYN/FIN/RST (in-kernel filter keeps volume low);
  always `XDP_PASS`. New diagram `xdp-capture`. Confirm: RingBuf in `#[xdp]`,
  network-types field names, TCP flags byte at offset 13, output matches
  `tcpdump 'tcp[tcpflags] & (…) != 0'`.
- **Ch 34 (xdp-lb)** — `#[xdp]` UDP port load balancer: `ptr_at_mut` to
  rewrite `udp.dest` round-robin across an `Array` of backends (cursor via
  `get_ptr_mut`), zero the optional IPv4 UDP checksum, `XDP_PASS`; per-backend
  `HITS`. New diagram `xdp-lb`. Confirm: `ptr_at_mut` writes pass the
  verifier, zeroed UDP checksum accepted end-to-end, `Array`
  `get/get_ptr_mut/set`, native-vs-SKB attach, even split. Documents the
  production gaps (return path / DSR / conntrack, incremental checksum,
  `XDP_REDIRECT` to remote backends) as forward work.
- New diagrams: 28 → 30. Both reuse the `ptr_at` discipline from Ch 32; Ch 34
  introduces packet **mutation** and map-driven backend selection.

### r18.1 — fix blank Fig 2.1 + lifecycle diagram (docs/diagrams only)
- Regenerated `lab-topology` (Fig 2.1): the previous hand-authored SVG had
  only its <defs> and no drawing body, so it rendered blank. Now generated
  with the diagram tool (host + Podman containers, target VM, peer VM, with
  scp/OTLP/traffic arrows).
- Replaced the ASCII-art lifecycle "picture" in Ch 5 with a real diagram,
  `ebpf-runtime-loop` (Fig 5.2): the runtime loop (hook fires → program
  writes map → loader reads → Grafana), complementing Fig 5.1's build→attach
  pipeline.
- Confirmed the Ch 2 `T=$(./vm-ip.sh …) && P=$(./vm-ip.sh …) && echo …`
  command is correct: vm-ip.sh prints only the IP (errors to stderr, exits
  non-zero), and the line runs in scripts/lab/ as set by the preceding cd.

### r19.0 — Part 4 (Networking) wrap-up: BPF_PROG_TEST_RUN + tcx — UNVERIFIED
- **Ch 35 (xdp-test)** — a `BPF_PROG_TEST_RUN` harness: load a small XDP
  filter (drop ICMP / pass else + PKTS counter), build synthetic Eth/IPv4
  packets in user space, run each through the program, assert the verdict,
  and check a map side-effect; exits non-zero on failure (CI-ready). New
  diagram `xdp-test-run`. KEY risk: the test-run binding — the example issues
  the bpf() command via a `libc::syscall` wrapper with a hand-laid `bpf_attr`
  test struct + `prog.fd()`; confirm against the installed Aya/kernel (or use
  Aya's own test_run if present).
- **Ch 36 (tcx)** — the Ch 31 classifier reattached via **tcx** (kernel 6.6+):
  no `qdisc_add_clsact`, `attach(&iface, TcAttachType::Ingress)` returns a
  bpf_link that auto-detaches on drop; cross-check shows `tcx/ingress` in
  `bpftool net show` with an empty `tc filter show` and no clsact qdisc. New
  diagram `tcx-chain`. Risk: exact Aya tcx attach API + link lifetime.
- **Networking part (Ch 27–36) is now complete.** New deps unchanged
  (network-types already in; Ch 35 adds `libc` to that one loader). Diagrams
  31 → 33.

### r20.0 — Part 5 (Security & LSM) opens: BPF LSM + signal programs — UNVERIFIED
- **Ch 37 (lsm-confine)** — first BPF LSM program: `#[lsm(hook="socket_connect")]`
  denies connect() for processes in a confined cgroup (return -EPERM),
  loaded against kernel BTF; teaches the allow/deny model, prior-verdict
  respect, and the `bpf` LSM enablement requirement. New diagram `lsm-decide`,
  new lab helper `scripts/lab/enable-bpf-lsm.sh`. Risks: Aya LSM API
  (`Lsm::load(hook,&btf)`/`attach`), `LsmContext` arg indexing (ret at 3),
  cgroup-id == dir inode.
- **Ch 38 (signal-kill)** — signal program: tracepoint on sys_enter_execve
  matches a forbidden filename prefix and calls `bpf_send_signal(SIGKILL)`
  to kill the process pre-exec; emits KillEvents over a RingBuf. New diagram
  `signal-kill`. Framed LAB-ONLY. Risks: bpf_send_signal from a tp, execve
  filename offset 16, bounded `starts_with`.
- Pulled Ch 38 (bpf_send_signal) into r20 alongside Ch 37 (plan had it in
  r21); iteration plan reconciled. Diagrams 33 → 35. Part 5 card now shows
  2 chapters.

### r21.0 — Part 5 (Security & LSM): offense + defense pair — UNVERIFIED
- **Ch 39 (pidhide)** — LAB-ONLY rootkit technique: two tracepoints on
  getdents64 (enter captures the user buffer, exit walks linux_dirent64 and
  splices out the target /proc/<pid> by extending the previous record's
  d_reclen via `bpf_probe_write_user`). Heavy detection section (kernel taint,
  bpftool prog show, /proc vs real task list). New diagram `pidhide`. Risks:
  getdents64 enter/exit offsets (dirent @24, ret @16), linux_dirent64 offsets
  (d_reclen @16, d_name @19), bpf_probe_write_user permission, bounded walk.
- **Ch 40 (lsm-fileprotect)** — defensive counterpart: `#[lsm(hook=
  "inode_permission")]` denies MAY_WRITE on one protected inode (read-only
  even for root), reading `inode->i_ino` via `bpf_probe_read_kernel` at a
  hard-coded `I_INO_OFFSET` (flagged version-specific; CO-RE in Part 9 fixes
  it). Fail-open on read error. New diagram `lsm-file-protect`. Risks: arg
  indexing (inode@0/mask@1/ret@2), MAY_WRITE value, i_ino offset.
- Diagrams 35 → 37. The hard-coded i_ino offset is the canonical CO-RE
  motivation; forward-ref to Ch 56 reinforced.

### r21.1 — fix blank Ch 39 body (rendering bug)
- Root cause: _includes/excalidraw.html emitted `{{ include.alt }}` as raw,
  unescaped HTML fallback text inside <object>. Ch 39's alt contained
  `/proc/<pid>`; the browser parsed `<pid>` as an unclosed tag and swallowed
  the caption, links, and all prose after the figure (only the SVG showed).
- Fix: `{{ include.alt | escape }}` in both the aria-label attribute and the
  fallback text — protects every chapter from `<…>` in alt forever. Also
  tidied Ch 39's alt (`/proc/<pid>` → `/proc/PID`). Ch 39 was the only
  chapter with a tag-like alt. No content change to the chapter body.

### r22.0 — Part 5 (Security & LSM) closes: escalation offense + sensor capstone — UNVERIFIED
- **Ch 41 (sudoadd)** — LAB-ONLY: intercept sudo's read() (comm=="sudo"),
  overwrite the returned buffer with an injected NOPASSWD sudoers line via
  bpf_probe_write_user; on-disk file untouched. enter_read stashes buf/count,
  exit_read rewrites; payload (`<user> ALL=(ALL:ALL) NOPASSWD:ALL #`) from a
  map, write bounded to 64B. Detection section + LSM-can-block-the-primitive.
  New diagram `sudo-escalate`. Risks: read enter/exit offsets (buf @24,
  count @32, ret @16), comm match, payload/padding vs real sudoers layout.
- **Ch 42 (secsensor)** — defensive capstone: three tracepoints (execve,
  ptrace, setuid) emit a uniform SecEvent over one RingBuf; user side
  classifies type+severity → ebpf_sec_events_total{type,severity}. Frames
  observe-vs-shield (telemetry broad/cheap; enforce narrow via LSM). New
  diagram `security-sensor`. Risks: tracepoint names (esp. setuid vs
  setreuid/setresuid), one RingBuf shared by three programs.
- **Security & LSM part complete (Ch 37–42):** confine, react, hide, protect,
  escalate, sense. Pulled Ch 42 into r22 (plan had it as r23); plan
  reconciled, the stray r23 row folded in. Diagrams 37 → 39.

### r23.0 — Part 7 (Schedulers) opens: sched_ext + struct_ops — UNVERIFIED
- **Ch 43 (scx-simple)** — sched_ext/struct_ops model taught via the real
  scx_simple callbacks (shown in C, faithful to upstream): select_cpu (idle
  fast path → SCX_DSQ_LOCAL), enqueue (→ SCX_DSQ_GLOBAL), init/exit,
  SCX_OPS_DEFINE; DSQs; the safety watchdog (evict → fall back to default).
  HONEST framing callout: sched_ext today = BPF-in-C + Rust-userspace
  (scx/scx_utils); Aya kernel-side struct_ops is emerging, so we run the
  shipping scx_simple (Fedora scx-scheds) and OBSERVE it with an Aya
  sched_switch probe (per-CPU ctx-switch counter → ebpf_ctxsw_total{cpu}).
  New diagram `scx-simple`. Example 43-scx-simple = Aya observer (scx-watch)
  + reference/scx_simple.bpf.c (read-only) + runbook demo. Requirements:
  kernel ≥ 6.12, CONFIG_SCHED_CLASS_EXT. New part "Schedulers (sched_ext)".
- NOTE: first chapter to use C for the (reference) BPF side — deliberate and
  documented, because that's the current reality of sched_ext tooling.

### r24.0 — Part 7 (Schedulers): a realistic policy (scx_nest) — UNVERIFIED
- **Ch 44 (scx-nest)** — the nest policy: concentrate work on a small set of
  warm, high-frequency cores; primary nest + reserve, promote under load,
  demote when idle. Shows simplified nest_select_cpu (C, ref) using
  scx_bpf_pick_idle_cpu over cpumasks + bpf_cpumask_* kfuncs (kfunc forward-
  ref to Part 8). Same honest pattern: run the real scx_nest (scx-scheds) and
  OBSERVE it with an Aya per-CPU busy probe (sched_switch, attribute interval
  to busy if prev_pid != 0 → ebpf_cpu_busy_ns_total{cpu}; live busy% bar), so
  the nest is visible as a few hot cores. New diagram `scx-nest`. Example
  44-scx-nest = cpu-busy probe + reference/scx_nest.bpf.c (simplified) +
  moderate-load runbook. Risks: scx_nest availability, sched_switch prev_pid
  offset (24), idle==prev_pid 0.
- Schedulers part now has its two-chapter tour (minimal + realistic). Diagrams
  40 → 41.

### r25.0 — Part 8 (Application targets) opens: probing nginx — UNVERIFIED
- **Ch 45 (nginx-probe)** — uprobes on a containerized nginx's request
  functions: req_start (ngx_http_process_request) stamps STARTS keyed by the
  request pointer (ctx.arg(0), never dereferenced); req_done
  (ngx_http_finalize_request) computes elapsed → log2(us) HIST. Teaches the
  two real-target frictions: SYMBOLS (nm/objdump; distro nginx stripped →
  debuginfo/build-with-symbols) and CONTAINER NAMESPACES (attach via
  /proc/<worker-pid>/root/usr/sbin/nginx to reach the in-container inode).
  Example = Aya uprobe latency tool + multi-stage UBI Containerfile building
  nginx from source with --with-debug -g (keeps .symtab) + containerized
  runbook (podman build/run on target, drive load, resolve worker, attach).
  New diagram `nginx-uprobe`. Risks: nginx symbol presence, UProbe::attach
  signature (fn/offset/target/pid), /proc/<pid>/root resolution, ctx.arg(0)
  ABI. Reuses the funclatency (Ch18) histogram, keyed by request object.
- Diagrams 41 → 42. First real third-party app target.

### r25.1 — docs sync: Ch5 chapter-shape + outline tables + CO-RE refs
- Ch 5 "shape of every chapter" rewritten to match the real chapter structure
  (concept+diagram → How the code works → Build/deploy/observe → Cross-check →
  What you learned → Verification status), instead of the old 6-step "recipe"
  that listed three separate crate sections (they're one "How the code works"
  section) and omitted the diagram and recap. Noted -common is per-chapter
  (when a shared record exists), and that the book never self-promotes.
- Fixed the outline tables in Ch 0 and PRD: Security & LSM grew to 6 chapters
  (37–42), so everything after shifted +1 — Schedulers 43–44, Application
  targets 45–46, Advanced kernel surface 47–56, Operating eBPF 57–61. PRD
  Security row description updated to its 6 chapters.
- De-numbered the CO-RE forward-refs (were "Chapter 56"/"Ch 56" in Ch 7, 8,
  15, 27) to "Part 9" so they can't drift as earlier parts grow.

### r25.2 — docs: make the Grafana side of "observe" real (and referenced)
- Ch 3: replaced the ASCII data-path block under "How Rust eBPF programs
  report in" with a new diagram `reports-in` (Fig 3.2) showing the opensnoop
  example fanning out to TWO faces of output — a live terminal table and
  `ebpf_*` metrics pushed via OTLP to otel-lgtm/Grafana. Added a "Where your
  output shows up" subsection that fixes the terminal-vs-Grafana distinction
  and gives a concrete Grafana viewing recipe (127.0.0.1:3000 → Explore →
  Prometheus → query `ebpf_`). Strengthened "Leave it running" to state every
  later chapter feeds this stack, and updated the closing checklist.
- Disambiguated chapter "Build, deploy, observe" steps so "watch X" is clearly
  the terminal and the `ebpf_*` metric is clearly Grafana: Ch 44 (per-CPU
  busy % in terminal; `ebpf_cpu_busy_ns_total{cpu}` rate() in Grafana) and
  Ch 43 (`ebpf_ctxsw_total{cpu}`). Ch 45 already named its Grafana metric.
- CONVENTION going forward: every program chapter's "Build, deploy, observe"
  names both the terminal live view and the specific `ebpf_*` metric to chart
  in Grafana. Older chapters (6–42) still describe terminal output well but
  mostly don't name the Grafana panel — a focused "observe pass" can retrofit
  them; tracked here as outstanding.

### r25.3 — observe pass: every chapter now names its Grafana view
- Renamed 5 stray metrics to the ebpf_ prefix so ALL series are discoverable
  under `ebpf_`: ebpf_function_latency_ms (18), ebpf_runqueue_latency_us (21),
  ebpf_memleak_outstanding_bytes (24), ebpf_bio_sequential_ratio (25),
  ebpf_estimated_power_watts + ebpf_system_power_watts (26).
- Added an "In Grafana" line to the "Build, deploy, observe" section of every
  program chapter (06–45): the exact PromQL to graph, shaped per metric type —
  counters → rate(); gauges (21,24,25,26) → raw; the funclatency histogram (18)
  → histogram_quantile/heatmap; shared ebpf_events_total chapters → filter by
  the chapter's `ebpf-*` service + sum by(program). 35 (BPF_PROG_TEST_RUN) is
  noted terminal-only. 43/44/45 already had theirs (r25.2 / r25.0).
- Enhanced the auto-provisioned dashboard (ebpf-overview.json, already mounted
  into otel-lgtm) into the universal front door: added a "Metric (explorer)"
  query variable over metrics(ebpf_.*) feeding a rate panel (counters) and a
  raw panel (gauges), alongside the existing events + logs panels. Referenced
  from Ch3 and the example README. (No manual import — it loads on stack up.)
- The earlier "watch X — where?" gap is now closed end to end: terminal view
  for the live glance, named ebpf_* metric/query for Grafana, one dashboard
  that browses them all.

### r26.0 — capstone: tying the three signals together (Ch 46) — UNVERIFIED
- New **Ch 46 (three-signals)**, a short capstone in the Application targets
  part. Core lesson: Java (JIT) and Python (interpreted) have no app symbols to
  uprobe, so instrument at the kernel SOCKET layer — the OBI insight. kprobes
  on tcp_recvmsg/tcp_sendmsg keyed by struct sock* (as u64), bpf_get_current_comm
  to tag the service, ring buffer of Req{dur_ns,comm}. Loader mints ONE trace_id
  per request and emits three correlated signals: span (Tempo), log (Loki), RED
  metrics (Prometheus) — finally exercising the trace/log/metric correlation the
  Ch3 datasources provisioned. Honest seams flagged: loader-minted trace_id is
  intra-service (not distributed W3C propagation); exemplar (metric→trace) hop
  may be missing in the Rust metrics SDK; recv/send pair is a simplification of
  real L7 parsing. Positions OBI (OpenTelemetry eBPF Instrumentation, donated
  Beyla) as the production tool, with its architecture diagram + docs link.
- Example 46-three-signals: httpwatch-{ebpf,common,loader} + Java (plain JDK
  HttpServer) and Python (FastAPI) containerized services + demo. Two new
  diagrams: three-signals, obi-arch. New metrics: ebpf_http_server_requests_total,
  ebpf_http_server_duration_ms.
- RENUMBER (Security stayed 6, now capstone inserted): postgres 46→47;
  Application targets 45→47; Advanced kernel surface 47–56 → 48–57; Operating
  eBPF 57–61 → 58–62. Updated Ch0 + PRD tables, part blurb, Ch45 next-pointer,
  plan rows. CO-RE refs already de-numbered to "Part 9" so unaffected.
- HIGHEST-uncertainty code in the book (3-signal OTel SDK wiring); marked
  unverified with the specific API surfaces to confirm.
- A LARGER capstone is planned for the very end of the book to reinforce this.

### r27.0 — Application targets closes: probing postgres (Ch 47) — UNVERIFIED
- **Ch 47 (pg-probe)** — a multi-process database target. ONE uprobe on the
  postgres binary covers every backend (attach pid=None); the single-threaded
  backend pid is a clean per-connection key. uprobe/uretprobe on
  exec_simple_query → per-query latency + SQL text (bpf_probe_read_user_str_bytes
  of arg0, 128B bounded, PII note); uprobe/uretprobe on ProcSleep → lock-wait
  time (contention invisible from outside). Introduces USDT (query__start /
  lock__wait__start, .note.stapsdt) as the stable DB-observability API, with an
  honest note that Aya USDT attach is still maturing so we uprobe the wrapped
  functions. Cross-check against pg_stat_statements + pg_stat_activity (the DB's
  own books). New diagram pg-probe. Example = pg-probe-{ebpf,common,loader} +
  demo running stock postgres image + a lock-contention scenario. New metrics
  ebpf_pg_query_duration_ms, ebpf_pg_lock_wait_ms. Symbol/debuginfo caveat
  documented (stock images stripped; need debug symbols or --enable-dtrace).
- Application targets part (45 nginx, 46 capstone, 47 postgres) now complete.
  Next: Part 8 Advanced kernel surface (48+), opening with detach/pinning.

### r28.0 — Part 8 (Advanced kernel surface) opens: detach/pinning (Ch 48) — UNVERIFIED
- **Ch 48 (pin-demo)** — the lifecycle foundation. BPF programs/maps/links are
  refcounted by fds; loader exit frees them. Pinning to bpffs (/sys/fs/bpf)
  creates a named reference that keeps them alive. Map pinned via
  HashMap::pinned (LIBBPF_PIN_BY_NAME) + EbpfLoader::map_pin_path; link pinned
  via take_link → FdLink::try_from → FdLink::pin; read back elsewhere via
  MapData::from_pin. pinctl load/read/detach demonstrates: load+pin+exit, the
  program keeps counting execs, a fresh process reads the pinned map, rm the
  pin detaches. Cross-check with bpftool prog/link/map show + map dump pinned.
  New diagram `pinning`. New metric ebpf_pinned_execs_total. Decoupling the
  eBPF lifecycle from the loader → forward-ref to Part 9 (zero-downtime).
- Plan Phase 9 rows resynced to current numbering (Advanced = 48–57). Next:
  Ch 49 syscall programs.

### r29.0 — Advanced surface: syscall programs (Ch 49) — UNVERIFIED (conceptual)
- **Ch 49 (syscall-prog)** — BPF_PROG_TYPE_SYSCALL: attaches to nothing, run via
  BPF_PROG_RUN, calls bpf() itself via bpf_sys_bpf (sleepable; bpf_sys_bpf/
  bpf_sys_close/bpf_btf_find_by_name_kind). Teaches loader programs ("strace of
  libbpf" replayed in-kernel) and light skeletons (bpftool gen skeleton -L
  embeds a generated syscall/loader program + data → no libelf/libbpf at load;
  path to signed loading). HONEST frame: Aya IS the user-space loader, so this
  is mechanism-to-recognize, not a tool to wield; aya-ebpf syscall-program
  authoring is experimental. Concept chapter: new diagram syscall-prog +
  illustrative/loader_program.rs (read-only sketch, like sched_ext reference)
  + demo using bpftool gen skeleton -L on a real built object. No ebpf_ metric
  (inspection chapter, like Ch35) — no In-Grafana line, by design.
- Next: Ch 50 user ring buffer (user space → BPF direction).
