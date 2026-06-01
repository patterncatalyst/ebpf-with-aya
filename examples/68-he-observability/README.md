# 68 — Observing a homomorphic-encryption workload (capstone addendum)

Time each homomorphic operation of a [TFHE-rs](https://github.com/zama-ai/tfhe-rs)
workload with eBPF — **without ever reading its (encrypted) operands**. This is the
Chapter 63 capstone machinery (uprobes + OTLP to Grafana) pointed at a workload
that is private by design.

## Why this is interesting

Homomorphic encryption computes on ciphertext without decrypting, so the server
never sees the data. That makes ordinary "log the inputs" observability both
impossible (the operands are ciphertext) and forbidden (the operator is the
threat model). eBPF fits because it measures *behavior* — how long each operation
takes, where time pools — not *values*. The observer here learns the operation
name and two timestamps per call, and nothing else.

## Layout

```
he-common/         Sample { op, dur_ns } — shared, #[repr(C)], no operands
he-observer-ebpf/  one uprobe (he_enter) + one uretprobe per he_* boundary
he-observer/       loader: attach the probes, drain the ring, record a histogram
he-workload/       TFHE-rs workload; he_keygen/he_encrypt/he_compute/he_decrypt
                   are #[no_mangle] #[inline(never)] pub extern "C" boundaries
```

## Run it

```bash
cd examples/68-he-observability && ./demo.sh
```

`demo.sh` builds both binaries, ships them to `ebpf-target`, schedules the
workload to start a few seconds late (so the observer attaches first and catches
the one-shot `he_keygen`), then attaches the observer to the workload's `he_*`
symbols.

- **Terminal:** the observer prints each operation and its duration in ms;
  `keygen` and `compute` dominate `encrypt`/`decrypt` — the signature of an FHE
  workload.
- **Grafana:** graph `ebpf_he_op_latency_seconds` as a heatmap and split by the
  `op` label (`keygen` / `encrypt` / `compute` / `decrypt`).

## Cross-check

```bash
[vm]$ sudo /usr/share/bcc/tools/funclatency -p "$(pgrep he-workload)" 'he_compute'
[vm]$ sudo bpftrace -e 'uprobe:/home/fedora/he-workload:he_compute { @=hist(nsecs); }'
[vm]$ sudo /usr/share/bcc/tools/profile -p "$(pgrep he-workload)" 5   # time pools in the NTT
```

## Attach points (the fragile bit)

uprobes attach to **symbols**, and release builds inline/monomorphize library
calls away — so the workload defines its own non-inlined, unmangled boundaries
(`#[no_mangle] #[inline(never)] pub extern "C"`). Confirm they survived:

```bash
[vm]$ nm /home/fedora/he-workload | grep -E 'he_(keygen|encrypt|compute|decrypt)'
```

If a symbol is missing, the corresponding `attach(Some("..."), ...)` will fail.

## Status

**Unverified** — written against the documented TFHE-rs and Aya APIs; not built
or run on hardware. Things to confirm on a real Fedora 44 run: the `tfhe` crate
version/features build (you may need arch-specific features), the `he_*` symbols
are present (`nm`), the uprobe/uretprobe pairs attach, and the histogram
populates per `op`. **License:** TFHE-rs is free for development, research, and
prototyping under Zama's BSD-3-Clause-Clear license; commercial use requires
Zama's patent license.
