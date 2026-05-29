---
title: "Hello, eBPF"
order: 6
part: Foundations
description: A first Aya program — a tracepoint that counts process executions in the kernel and reports to Grafana — with a short libbpf / libbpf-rs warm-up first, deployed to the target VM.
duration: 30 minutes
---

Everything so far has been setup. This chapter is the first program:
an eBPF tracepoint that counts every `execve` on the target VM, written
in Rust with Aya, deployed to the guest, and reporting its count into
Grafana. It's the smallest thing that exercises the *entire* loop —
build, deploy, load, attach, map, report — so that every later chapter
is just a variation on a path you've already walked.

We start with a brief libbpf/libbpf-rs warm-up, because seeing the C
mental model once makes Aya's design choices legible. Then we switch to
Aya for this program and everything after it.

The code is in `examples/06-hello-world/`.

## Warm-up: the libbpf mental model

The original eBPF toolchain is **libbpf** (C). You write a `.bpf.c`
file with a `SEC("tracepoint/...")` annotated function, compile it to a
BPF object with `clang -target bpf`, generate a **skeleton** header,
and a user-space C program loads the skeleton, attaches it, and reads
maps. **libbpf-rs** is a Rust wrapper over libbpf: the kernel side is
still C compiled by clang, but the user side is Rust, with a generated
skeleton you call from `main.rs`.

The shape — *BPF object on one side, a loader that attaches it and
reads maps on the other* — is identical to what you saw in Chapter 5.
The thing libbpf established and we keep is **CO-RE**: that
`SEC(".maps")` definition plus BTF relocations is what lets one object
run across kernels.

What Aya changes: the kernel side is **also Rust** (`no_std`, compiled
to the BPF target), so there's no C, no clang invocation you maintain,
no separate skeleton-generation step — and the same Rust safety that
keeps the verifier happy. We use libbpf-rs nowhere else in this
tutorial; it's here once so the lineage is clear. From now on it's
Aya.

> If you want to *see* libbpf working without writing C, the
> Fedora-packaged `bcc-tools` you installed in the VM are libbpf/BCC
> programs. `sudo execsnoop-bpfcc` on the target is, functionally, the
> C version of the program you're about to write in Rust. Comparing
> the two is instructive.

## The program, in three crates

Recall the workspace shape from Chapter 4. Hello-world fills it in:

- **`hello-ebpf/`** — the kernel program. A `#[tracepoint]` handler
  named `hello` that, on each `execve`, bumps a per-CPU counter and
  emits one `aya-log` line.
- **`hello-common/`** — shared constants (the counter's index and
  length) so both halves agree.
- **`hello/`** — user space. Loads the embedded object, initializes
  `aya-log`, attaches the tracepoint to `syscalls:sys_enter_execve`,
  then every second sums the per-CPU counter and exports the growth as
  the OTLP metric `ebpf_events_total`.

### The kernel half

The handler is tiny. The whole program is in
`hello-ebpf/src/main.rs`; the heart of it:

```rust
#[map]
static EVENTS: PerCpuArray<u64> = PerCpuArray::with_max_entries(EVENTS_LEN, 0);

#[tracepoint]
pub fn hello(ctx: TracePointContext) -> u32 {
    if let Some(counter) = EVENTS.get_ptr_mut(EVENTS_INDEX) {
        unsafe { *counter += 1; }
    }
    info!(&ctx, "hello: execve observed");
    0
}
```

Three things to notice. The `#[map]` static *is* the kernel map — Aya
turns it into a real `BPF_MAP_TYPE_PERCPU_ARRAY` at load. The
`#[tracepoint]` macro marks `hello` as a tracepoint program the loader
can find by name. And `info!` is `aya-log-ebpf`: the kernel can't write
to Grafana, but it can emit a log record that user space forwards.

The file also carries a `license` section declaring `Dual MIT/GPL` —
required for the GPL-only helpers `aya-log` uses — and a panic handler
(`no_std` requires one; newer `aya-ebpf` may provide it, in which case
you delete ours).

### The user-space half

`hello/src/main.rs` is ordinary async Rust. Loading and attaching:

```rust
let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(
    env!("OUT_DIR"), "/hello"
)))?;
EbpfLogger::init(&mut ebpf).ok();

let program: &mut TracePoint = ebpf.program_mut("hello").unwrap().try_into()?;
program.load()?;
program.attach("syscalls", "sys_enter_execve")?;
```

`include_bytes_aligned!` embeds the BPF object that `build.rs` produced
(via `aya-build`) into the binary — that's the "one self-contained
artifact" that makes deployment a single `scp`. `program_mut("hello")`
finds the program by the function name from the kernel crate, `load()`
runs it past the verifier, and `attach(...)` wires it to the
tracepoint.

Reading the map and reporting:

```rust
let events: PerCpuArray<_, u64> = PerCpuArray::try_from(ebpf.map_mut("EVENTS").unwrap())?;
loop {
    sleep(Duration::from_secs(1)).await;
    let total: u64 = events.get(&EVENTS_INDEX, 0)?.iter().copied().sum();
    let delta = total.saturating_sub(last_total);
    if delta > 0 { counter.add(delta, &[KeyValue::new("program", "hello")]); last_total = total; }
}
```

Summing across CPUs is why we used a *per-CPU* array — no locking in
the kernel hot path, and user space does the cheap aggregation. The
delta feeds the same `ebpf_events_total` series the Python client used
in Chapter 3, so they share a panel.

## Build, deploy, observe

With the Chapter 3 stack up and the Chapter 2 `ebpf-target` running,
the whole loop is one script:

```bash
cd examples/06-hello-world && ./demo.sh
```

`demo.sh` builds the release binary on the host, resolves the libvirt
gateway address (so the guest can reach the host's OTLP port), ships
the binary to the target with the Chapter 2 `deploy-to-target.sh`, and
runs it under `sudo`. Leave it running and, in another terminal,
generate some executions on the target:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'for i in {1..50}; do /bin/true; done'
```

Open the **eBPF with Aya — Overview** dashboard
(`http://127.0.0.1:3000/d/ebpf-overview`). `ebpf_events_total` climbs
by ~50, and the log panel shows the kernel's `hello: execve observed`
lines forwarded by `aya-log`. You just watched a program you wrote run
*in the guest kernel* and report to your laptop.

## Cross-check against the kernel

Never fully trust your own reporting until an independent tool agrees.
On the target VM:

```bash
[vm]$ sudo bpftool prog list | grep -A3 tracepoint && sudo bpftool map dump name EVENTS
```

```bash
[vm]$ sudo bpftrace -e 'tracepoint:syscalls:sys_enter_execve { @=count(); }'
```

`bpftool map dump` shows the raw per-CPU counter the kernel is writing;
`bpftrace` gives a fully independent count. If your Grafana number, the
`bpftool` dump, and the `bpftrace` count all move together, the program
is correct end to end. If they diverge, the divergence tells you which
half is wrong — which is exactly the debugging leverage Chapter 5
promised.

## When the build doesn't compile

It might not, the first time — and that's expected, not a failure of
the tutorial. Aya's templates have moved between an `xtask` build and
the `aya-build` `build.rs` approach used here, and the
`opentelemetry` crate's exporter API shifts between minor versions. If
`cargo build` errors:

1. Generate the canonical scaffold for *your* installed Aya with
   `cargo generate --name hello https://github.com/aya-rs/aya-template`
   and compare its `build.rs`, `Cargo.toml` versions, and program
   skeleton against this one. Prefer the generated wiring; port this
   program's logic into it.
2. For OTLP errors, check the `opentelemetry-otlp` docs for the current
   `MetricExporter` / `PeriodicReader` builder names and adjust
   `init_otel()`.

Then record the result — pass or the fix you needed — in the
reconciliation plan. This is the test-on-real-hardware loop the whole
project is built around, and it's how this chapter graduates from
<span class="status status--unverified">unverified</span> to
<span class="status status--verified">verified (Fedora 44)</span>.

## End of Foundations

You now have the complete loop: a lab to deploy into, a stack to report
to, a toolchain to build with, the concepts to reason about programs,
and a first program proving it all works together. Every chapter from
here is a new program type and a new thing to measure — `kprobe` and
`unlink` next — but the machinery is the machinery you just built.

See the [roadmap]({{ "/plans/iteration-plan/" | relative_url }}) for the
full chapter list and which iteration ships each one.

---

*Verification status: <span class="status status--unverified">unverified</span>.
See the "When the build doesn't compile" section — the first build on a
real Fedora 44 target is the verification step.*
