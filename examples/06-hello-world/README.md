# Example 06 — Hello, eBPF (with Aya)

Your first Aya program: a tracepoint on `sys_enter_execve` that counts
process executions in the kernel and reports the count to Grafana via
OpenTelemetry.

## Layout

```text
hello-common/   # shared no_std constants/types (EVENTS_INDEX, EVENTS_LEN)
hello-ebpf/     # the in-kernel program: tracepoint handler -> per-CPU counter + aya-log
hello/          # user space: load, attach, read map, export OTLP metric
  build.rs      # compiles hello-ebpf for the BPF target (aya-build)
Cargo.toml      # workspace
rust-toolchain.toml
demo.sh         # build -> deploy to VM -> run -> observe
```

## Run it

Prereqs: Chapter 4 toolchain on the host, Chapter 2 `ebpf-target` VM
up, Chapter 3 stack running.

```bash
./demo.sh build     # just build on the host
./demo.sh           # build + deploy to the VM + run (Ctrl-C to stop)
```

Generate some execve traffic on the target so the counter moves:

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'for i in {1..50}; do /bin/true; done'
```

Then watch `ebpf_events_total` climb on the **eBPF with Aya — Overview**
dashboard at http://127.0.0.1:3000/d/ebpf-overview.

## Cross-check with Fedora tooling (on the VM)

```bash
[vm]$ sudo bpftool prog list | grep -A3 tracepoint
[vm]$ sudo bpftool map dump name EVENTS
[vm]$ sudo bpftrace -e 'tracepoint:syscalls:sys_enter_execve { @=count(); }'
```

`bpftool map dump` should show the per-CPU counter incrementing; the
`bpftrace` one-liner is an independent count to compare against what
`hello` reports.

## ⚠ Verification status

**Unverified.** This code is written to current Aya conventions
(`aya` 0.13.x, `aya-ebpf` 0.1.x) but has **not** been compiled or run
in producing this iteration — there was no Fedora 44 target available
at authoring time. Treat the first `cargo build` as the test:

- If `build.rs`/`aya-build` wiring differs from your generated
  `aya-template` (templates have churned between an `xtask` approach
  and the `aya-build` approach), prefer the structure your
  `cargo generate` produces and port this program's logic into it.
- The OTLP exporter API (`opentelemetry` 0.27) moves between minor
  versions; if it doesn't compile, check the crate docs for the
  current `MetricExporter`/`PeriodicReader` builder names.

Record the outcome in `_plans/reconciliation-plan.md`. The
test-on-real-hardware loop is exactly how this program graduates from
`unverified` to `verified (Fedora 44)`.
