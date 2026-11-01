---
title: "Verification log"
layout: plan
---

Verification-status footers removed from reader-facing chapters (site chrome shows reader content only; banked here, plans collection output:false). One section per chapter, verbatim.

## 01-prerequisites

*Verification status: <span class="status status--verified">verified — Fedora 44 host</span>.
These prerequisites were installed and used to build and run the whole corpus
during the smoke campaign (Rust nightly + `rust-src`, `bpf-linker`, libvirt/KVM,
podman, the LGTM stack). If something here doesn't match what you see on your
own machine, that gap is exactly what the
[reconciliation plan]({{ "/plans/reconciliation-plan/" | relative_url }})
exists to record — open an issue with your output.*

## 02-lab-setup

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Every command and script in this chapter was run to build the actual lab used to
verify the corpus: the `ebpf-target` / `ebpf-peer` guests provision from these
steps, boot with BTF present, and the daily lifecycle scripts
(`lab-up`/`lab-down`/`snapshot`/`revert`) work as described. The Fedora Cloud
image filename can still drift against the live mirror — see the callout above.
Record any deltas in the
[reconciliation plan]({{ "/plans/reconciliation-plan/" | relative_url }}).*

## 03-observability-stack

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
The stack, client, and dashboard were run end-to-end on Fedora 44
(kernel 7.1.3-200.fc44): `grafana/otel-lgtm:0.28.0` comes up under rootless
Podman, Grafana is reachable on `127.0.0.1:3000`, the Python 3.14 client
pushes over OTLP on `127.0.0.1:4318`, and its metrics, traces, and logs land
in the auto-loaded dashboard. The `otel-lgtm` tag (`0.28.0`) is pinned in
`compose.yaml`.*

## 04-rust-aya-toolchain

*Verification status: <span class="status status--verified">verified — Fedora 44 host</span>.
This toolchain built the entire corpus (~60 examples) on this Fedora host
throughout the smoke campaign: the nightly + `rust-src`/`build-std` path for the
kernel crate, `bpf-linker` (including the LLVM shared-lib fallback), the stable
loader toolchain, and the per-crate `rust-toolchain.toml` pins all work as
described. The gdbserver remote-debug path (RustRover / gdb) was exercised
end-to-end. Aya crate versions: aya 0.14.x, aya-ebpf 0.2.x.*

## 06-hello-world

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 07-kprobe-unlink

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 08-fentry-unlink

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 09-opensnoop

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 10-sigsnoop

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 11-execsnoop

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 12-exitsnoop

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 13-bashreadline

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 14-uprobe-rust

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 15-btf-uprobe

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 16-container-targets

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 17-sslsniff

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 18-funclatency

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 19-goroutine-states

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 20-javagc

*Verification status: <span class="status status--verified">verified</span>
— Fedora 44, kernel 7.1.3, OpenJDK 26 (Red Hat build). Confirmed the stock
JDK ships **no** hotspot gc USDT markers, then resolved
`_ZN15G1CollectedHeap32do_collection_pause_at_safepointEm` from `.symtab`,
attached uprobe+uretprobe via aya 0.14, and streamed real GC pauses from
the VM Thread (~0.4–6 ms) exported as `jvm_gc_pause_ms`.*

## 21-runqlat

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 22-hardirqs

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 23-profile

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 24-memleak

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 25-biopattern

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 26-energy

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 27-tcpconnlat

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 28-tcpstates

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 29-http-l7

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): the socket filter loads and parses cross-host HTTP request/response lines. Confirmed on this kernel.*

## 30-sockops

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 31-tc-classify

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 32-xdp-drop

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 33-xdp-capture

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 34-xdp-lb

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 35-xdp-test

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 36-tcx

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 37-lsm-confine

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 38-signal-kill

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 39-pidhide

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built on the host, run on the lab VM. Behaviorally confirmed, not just "loads":
with the target PID attached, a bare `ls /proc` no longer lists it (hide count
increments) while `kill -0 <pid>` succeeds and `/proc/<pid>/` is still directly
statable; on detach the PID reappears. The scan bound was raised from 64 to 512
after the original cap scanned only the first 64 of `/proc`'s 200+ single-call
`getdents64` entries and thus hid nothing. Also confirmed on this kernel:
`bpf_probe_write_user` does not taint (`/proc/sys/kernel/tainted` stays `0`).*

## 40-lsm-fileprotect

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 41-sudoadd

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3, sudo 1.9.17p2</span>.
Built on the host, run on the lab VM. Detached, `victim` cannot sudo; while
attached, `sudo -u victim sudo -n id` returns `uid=0(root)` on the first
attempt and sudo itself stays healthy; on detach `victim` is denied again and
`/etc/sudoers` on disk is unchanged. The original program was rejected by this
kernel's verifier (`bpf_probe_write_user` size is `ARG_CONST_SIZE` → the
possibly-zero length failed as "R3 invalid zero-sized read"); the size is now
clamped to `1..=64`. Targeting was moved from `comm`-only to a sudoers-header
signature after the `comm`-only version corrupted sudo's shared-library reads
and missed the `lseek`-rewound parse read (both detailed above).*

## 42-secsensor

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 43-scx-simple

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 44-scx-nest

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 45-nginx-probe

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 46-three-signals

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 47-pg-probe

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 48-pin-demo

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 49-syscall-prog

*Verification status: <span class="status status--verified">verified</span>
— Fedora 44, kernel 7.1.3 (bpftool v7.6.0, libbpf 1.6). `bpftool gen skeleton
-L` on a libbpf-style C object (`reference/skel_demo.bpf.c`, compiled on the
target) emits the `struct bpf_loader_ctx` / `skel_internal.h` light skeleton —
the BPF_PROG_TYPE_SYSCALL loader path — while the full skeleton emits the
classic `libbpf.h` / `bpf_object_skeleton` ELF path. Note: aya-ebpf objects
can't be used here — they carry legacy `maps`-section definitions that libbpf
v1.0+ rejects. The aya-ebpf loader-program sketch stays illustrative —
syscall-program authoring in aya is experimental.*

## 50-user-rb

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): user space produces 1000 samples and the BPF program drains them (count=1000, sum=500500). Uses aya 0.14 + aya-ebpf 0.2.*

## 51-userspace-ebpf

*Verification status: <span class="status status--verified">verified</span>
— host run (no VM, no root). `cargo run --release` builds against the current
`rbpf` API (`assembler::assemble`, `EbpfVmRaw::new`, `register_helper`,
`execute_program`, `jit_compile` / `execute_program_jit`) and the interpreter
and JIT agree: `interpreter=42 jit=42`. The JIT path is x86-64 only.*

## 52-kfunc-task

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): tallies found (target pid running) vs missing. The kfunc form (bpf_task_from_pid) is not expressible in aya-ebpf (see below); this checks the current task instead.*

## 53-bpf-token

*Verification status: <span class="status status--verified">verified (privileged half) — Fedora 44, kernel 7.1.3</span>.
On the lab VM, bpffs accepts `delegate_cmds`/`delegate_maps`/`delegate_progs`
and shows them in `mount`; a **bogus** axis is rejected, proving the kernel
enforces the policy; and `BPF_TOKEN_CREATE` is present in the kernel ABI.
(`bpftool feature probe` v7.6.0 shows no "token" line — a bpftool gap, not a
kernel one.) The Aya `EbpfLoader` token option is confirmed **still emerging**:
against the pinned aya 0.14.0 there is no `token_path`/token method on the
loader, so `illustrative/loader_with_token.rs` shows the intended shape, not a
shipped API. libbpf threads it via `bpf_token_path` today.*

## 54-bpf-timer

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): reports a per-second event rate. The in-kernel bpf_timer form is not expressible in aya-ebpf (see below); the rate is computed in the userspace loader.*

## 55-structops-cc

*Verification status: <span class="status status--verified">verified</span>
— Fedora 44, kernel 7.1.3 (clang 22, bpftool v7.6.0). The CC algorithm compiles
against this kernel's `vmlinux.h` and `bpftool struct_ops register` installs it:
`bpf_reno` appears in `tcp_available_congestion_control` alongside reno/cubic.
One fix was needed against current BTF — `tcp_slow_start` now returns `__u32`
(not `void`) in `vmlinux.h`, so the `extern` had to match or clang errors with
"conflicting types". The exact `tcp_sock` field names
(`snd_cwnd`/`snd_ssthresh`/`prior_cwnd`) resolve on this kernel; treat the aya-ebpf
struct_ops rendering as emerging.*

## 56-dynptr-arena

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 57-bpf-iter

*Verification status: <span class="status status--verified">verified</span>
— Fedora 44, kernel 7.1.3 (clang 22, bpftool v7.6.0). The task iterator compiles
against this kernel's `vmlinux.h`, `bpftool iter pin` pins it, and `cat`ing the
pin produces the process table (469 tasks) with `tgid`/`pid`/`comm` — assembled
entirely in the kernel. The aya-ebpf iterator rendering stays emerging — the C
path is canonical.*

## 58-core

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 59-bpf-lifecycle

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 60-offload

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 61-power

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 62-correlation

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 63-capstone

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM host (Fedora 44): both services build and come up under podman-compose, and one traceparented request flows FastAPI /checkout -> Quarkus /inventory across both services.*

## 64-bpftrace-python

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 65-bpftool-python

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 66-bcc-tools

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 67-retrospective

*Verification status: this retrospective is reflection, not code — but it points
back at sixty-seven chapters <span class="status status--verified">verified on
Fedora 44 (kernel 7.1.3)</span>, run end-to-end on the lab. Your own run, on your
own kernel, is the final check — and the point. Thank you for building this with
me.*

## 68-he-observability

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*

## 69-pcp-ebpf

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
