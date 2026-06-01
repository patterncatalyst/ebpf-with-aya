---
title: "Capstone addendum: observing a homomorphic-encryption workload"
order: 68
part: Addenda
description: "A capstone extension. Homomorphic encryption lets a server compute on data it can never see — which makes ordinary observability (log the inputs, trace the values) both impossible and forbidden. eBPF is the right fit precisely because it measures timing and resource behavior without touching operands. We attach uprobes to a TFHE-rs workload's operation boundaries, build a per-operation latency histogram, and surface it in Grafana — observing the system while the data stays sealed."
duration: 40 minutes
---

The Chapter 63 capstone followed one request through two services and the kernel,
joining everything on a single `trace_id`. This addendum takes the same machinery
— uprobes, timing, OTLP to Grafana — and points it at a workload where you
deliberately *cannot* and *must not* see the payload: **homomorphic encryption**.
It's a small chapter with an outsized point about what observability *is*. When
the entire value of a system is that the operator never sees the data, your
telemetry has to measure the system without measuring the secret — and eBPF,
which times function boundaries and samples stacks rather than reading operands,
turns out to be exactly the tool for that constraint.

The code is in `examples/68-he-observability/`. `examples/68-he-observability/demo.sh`
builds a small TFHE-rs workload and an Aya uprobe observer, runs them on the VM,
and shows a per-operation latency histogram; the
`examples/68-he-observability/README.md` covers the details.

{% include excalidraw.html
   file="he-observability"
   alt="Observe a privacy-preserving workload: time the operations, never the data. On the left, an HE workload built on TFHE-rs in Rust exposes boundary functions — he_keygen, he_encrypt which produces ciphertext, he_compute which does homomorphic add and multiply on ciphertext, and he_decrypt — and its operands stay encrypted. An eBPF/Aya observer attaches a uprobe and uretprobe to each he_* boundary by symbol, measuring entry-to-return duration. That feeds a metric, ebpf_he_op_latency_seconds tagged by op (keygen, encrypt, compute, decrypt), exported to Grafana. It extends the Chapter 63 capstone with the same uprobe and OTel machinery, on a workload you deliberately cannot read: eBPF measures duration and stacks, not values."
   caption="Figure 68.1 — timing each homomorphic operation by symbol, without ever reading its (encrypted) operands" %}

## What homomorphic encryption is

Normally, to compute on data a server must first decrypt it — so the server sees
the plaintext. **Homomorphic encryption (HE)** breaks that requirement: it lets
you run computations directly on ciphertext such that decrypting the result
gives the same answer as if you'd computed on the plaintext. The data goes in
encrypted, is operated on while still encrypted, and comes back encrypted; the
machine doing the work never holds the cleartext at all.

There's a ladder of capability. *Partially* homomorphic schemes support one
operation unboundedly (RSA is multiplicatively homomorphic; Paillier is
additively so). *Somewhat* or *leveled* schemes support both addition and
multiplication but only to a bounded circuit depth. **Fully homomorphic
encryption (FHE)** supports arbitrary computation, and the trick that makes it
"fully" is **bootstrapping** — a periodic, expensive refresh that resets the
accumulated noise so computation can continue indefinitely. Modern schemes rest
on the hardness of lattice problems — Learning With Errors (LWE) and its ring
variant — which are believed secure even against quantum computers. The example
here uses **TFHE-rs**, Zama's pure-Rust implementation of the TFHE scheme
(fully homomorphic encryption over the torus), which fits this book's Rust
through-line: the workload you observe is itself a Rust binary.

The catch — and the reason observability matters — is cost. Every ciphertext is
far larger than its plaintext, noise grows with each operation, bootstrapping is
heavy, and the inner loop is dominated by large polynomial multiplications
(number-theoretic transforms). Homomorphic operations routinely run **thousands
to millions of times slower** than the plaintext equivalent. A privacy-preserving
service lives or dies on *where* that time goes — which operation, how often
bootstrapping fires — and you cannot answer that without measuring it.

## Why you want metrics without observing the data

Here is the principle the chapter is really about. In an ordinary service you
debug by looking at values: log the request fields, put the user id on the span,
inspect the payload. With HE that is **both impossible and forbidden**:

- **Impossible**, because the operands are ciphertext. There is no plaintext to
  read at the point of computation — that's the whole construction.
- **Forbidden**, because the threat model is *the operator*. HE exists for
  settings — confidential computing, regulated data, zero-trust outsourcing —
  where the party running the computation is explicitly not trusted to see the
  data. An observability layer that peeked at operands would defeat the entire
  reason the system uses HE.

So you need telemetry that is **data-blind by construction**: it must reveal how
the system *behaves* — latency per operation, which operation dominates, how
often bootstrapping runs, CPU and memory pressure — while revealing nothing about
the *values*. That is precisely the shape of eBPF observability. A uprobe on
`he_compute` records that the call took 84 ms; it does not, and structurally
cannot in this design, read the ciphertext bytes being multiplied. A CPU profile
(Chapter 23) records that the time went into the NTT routine; it captures stack
addresses, not operands. eBPF measures the *system*, not the *secret* — which is
usually framed as a safety nicety, but here is the only kind of observability the
problem permits.

This is the capstone's lesson inverted. There, the goal was to see *everything*
about one request, correlated. Here, the goal is to see everything about the
system's *performance* while seeing *nothing* about its data — and the same
eBPF instruments deliver both, because they were always measuring behavior rather
than content.

## How the code works

The workload (`he-workload`, using the `tfhe` crate) wraps each homomorphic
operation in a named, non-inlined boundary function — `he_keygen`, `he_encrypt`,
`he_compute`, `he_decrypt` — each marked `#[no_mangle] #[inline(never)] pub
extern "C"`. That is deliberate and is the same technique as the uprobe-on-Rust
chapter (Chapter 14): an optimized release build inlines and monomorphizes the
library's internals away, so you give the profiler stable symbols to attach to by
defining the boundaries yourself. Inside, the functions do real TFHE-rs work —
generate keys, encrypt a `FheUint8`, multiply two ciphertexts, decrypt — holding
state in a `static` so the signatures stay simple to probe.

The observer (`he-observer`) is a `funclatency`-style timer (Chapter 18) built
from **uprobe/uretprobe pairs**. On entry to any boundary it stamps the start
time keyed by `pid_tgid`; on return it computes the delta and emits a small
record tagged with the operation:

```rust
#[map] static START:  HashMap<u64, u64> = HashMap::with_max_entries(1024, 0);
#[map] static EVENTS: RingBuf           = RingBuf::with_byte_size(64 * 1024, 0);

fn on_entry() {                                   // every he_* entry
    let id = bpf_get_current_pid_tgid();
    let now = unsafe { bpf_ktime_get_ns() };
    let _ = START.insert(&id, &now, 0);
}
fn on_return(op: u32) {                            // each he_* return knows its op
    let id = bpf_get_current_pid_tgid();
    if let Some(&start) = unsafe { START.get(&id) } {
        let dur = unsafe { bpf_ktime_get_ns() }.saturating_sub(start);
        if let Some(mut e) = EVENTS.reserve::<Sample>(0) {
            unsafe { (*e.as_mut_ptr()) = Sample { op, dur_ns: dur }; }
            e.submit(0);
        }
        let _ = START.remove(&id);
    }
}
```

There's one `#[uprobe]` and one `#[uretprobe]` per boundary; the entry handlers
all share `on_entry`, and each return handler calls `on_return` with its own
operation id. Because the workload runs the operations sequentially on one
thread, keying the in-flight start time by `pid_tgid` is sufficient — there's
never more than one boundary open at a time per thread. The user-space loader
attaches each program to its symbol on the workload binary with
`UProbe::attach(Some("he_compute"), 0, &target, None)` (and the matching
uretprobe), drains the ring buffer, and records each `dur_ns` into an
`f64_histogram` named `ebpf_he_op_latency_seconds` with an `op` label — the same
OTLP wiring as every other chapter. Note what never appears in any of this: a
single operand. The observer sees a symbol, two timestamps, and an operation
name.

## Build, deploy, observe

```bash
cd examples/68-he-observability && ./demo.sh
```

The demo builds the TFHE-rs workload and the Aya observer, copies both to the
VM, starts the observer attached to the workload binary's `he_*` symbols, then
runs the workload (keygen once, then a loop of encrypt → compute → decrypt). The
**terminal live-view** is the observer printing each operation and its duration —
you'll see `keygen` and `compute` dwarf `encrypt`/`decrypt`, the signature shape
of an FHE workload. **In Grafana**, graph `ebpf_he_op_latency_seconds` as a
heatmap and break it down by the `op` label to see the per-operation latency
distribution; `compute` (and bootstrapping inside it) is where the milliseconds
live. The dashboard tells you everything about cost and nothing about content.

## Cross-check

```bash
[vm]$ sudo /usr/share/bcc/tools/funclatency -p "$(pgrep he-workload)" 'he_compute'   # same op, bcc's view
[vm]$ sudo bpftrace -e 'uprobe:/path/to/he-workload:he_compute { @=hist(nsecs); }'   # bpftrace's view
[vm]$ sudo /usr/share/bcc/tools/profile -p "$(pgrep he-workload)" 5                   # where the time goes (NTT)
```

`funclatency` on `he_compute` should track your `ebpf_he_op_latency_seconds`
histogram for that op, and `profile` should show the time pooling in the
library's polynomial-multiplication routines — independent confirmation that the
observer is timing the right thing, again without reading a byte of data.

## What you learned

- **Homomorphic encryption** computes on ciphertext without decrypting, so a
  server can process data it never sees — at a cost (bootstrapping, NTT-heavy
  polynomial math) of being orders of magnitude slower, which is exactly why its
  performance must be observed.
- That observability must be **data-blind by construction**: the operands are
  ciphertext (impossible to read) and the operator is the threat (forbidden to
  read), so you measure *behavior* — per-operation latency, where time pools —
  never *values*. eBPF fits because it times symbols and samples stacks, not
  operands.
- The implementation is the capstone's machinery unchanged: **uprobe/uretprobe**
  pairs on named workload boundaries (Chapter 14's technique), a `funclatency`
  histogram (Chapter 18), and OTLP to Grafana — now proving that the same tools
  which can see *everything* about a request can, by design, see *nothing* about
  a secret.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that the TFHE-rs workload builds and the `he_*`
boundary symbols survive optimization (verify with `nm`/`objdump`; mark them
`#[no_mangle] #[inline(never)]` and check they're present); that the
uprobe/uretprobe pairs attach to the workload binary path on the VM; that
`ebpf_he_op_latency_seconds` populates per `op`; and that `funclatency`/`profile`
agree. TFHE-rs is free for development, research, and prototyping under Zama's
license — review its terms before any commercial use.*
