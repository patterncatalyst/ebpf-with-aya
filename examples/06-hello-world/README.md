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

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on
the lab VM (Fedora 44, kernel 7.1.3-200.fc44): the program builds,
loads, attaches to `syscalls:sys_enter_execve`, and runs as described,
with the per-CPU counter incrementing and `ebpf_events_total` reported
to Grafana. Attach targets and struct offsets can be kernel-version
specific, so re-check them if you build against a different kernel. If
`build.rs`/`aya-build` wiring or the `opentelemetry` exporter API
differs on your toolchain, prefer the structure your `cargo generate`
produces and adjust the OTLP builder names to match your installed
crate versions.
