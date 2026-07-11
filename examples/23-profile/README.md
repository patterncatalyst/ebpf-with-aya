# Example 23 — profile (sampling CPU profiler)

Sample stacks across all CPUs and emit **folded** output — the input
format for flame graphs. This is *where* CPU time goes, not just how
long things wait.

## What this shows (new techniques)

- A **`perf_event`** program — a new program type that fires on a timer
  (here 99 Hz per CPU), set up from user space across all online CPUs.
  Cost is fixed (99 samples/sec/CPU) no matter how busy the machine is.
- **Stack walking**: capturing the live call stack with
  `bpf_get_stackid` into a **`StackTrace`** map — separately for kernel
  (`0`) and user (`BPF_F_USER_STACK`) stacks.
- Collapsing identical stacks via a count map keyed by
  `(pid, comm, kstack_id, ustack_id)`, then **symbolizing**: kernel
  frames via `/proc/kallsyms` (`aya::util::kernel_symbols`), user frames
  as addresses.
- Emitting **folded stacks** (`comm;frame;frame;… count`).

## Run it

```bash
./demo.sh build         # build on host
./demo.sh               # deploy, sample 10s, print folded stacks
SECS=30 ./demo.sh > out.folded     # longer run, capture to a file
```

Generate load on the VM while it samples:

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'timeout 10 sha256sum /dev/zero'
```

Turn the fold into a flame graph (host, with Brendan Gregg's
FlameGraph):

```bash
flamegraph.pl out.folded > cpu.svg
```

## Continuous profiling in the stack

The `otel-lgtm` stack bundles **Pyroscope**. The natural next step is to
push these profiles to Pyroscope (pprof format) for continuous profiling
and Grafana's flame-graph panel — wiring that ingest path is left as an
extension; this example emits the standard folded format that every
flame-graph tool accepts.

## Cross-check (on the VM)

```bash
[vm]$ sudo profile-bpfcc -F 99 10
[vm]$ sudo perf record -F 99 -a -g -- sleep 10 && sudo perf script | stackcollapse-perf.pl
```

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM (Fedora 44, kernel 7.1.3-200.fc44): the `perf_event` program builds,
loads, attaches across all online CPUs, and samples call stacks as
described, emitting folded output. Attach targets and struct offsets can be
kernel-version-specific. User-frame symbolization is intentionally left as
hex — wire in `blazesym` for names.
