---
title: "Retrospective: the whole arc, and the road ahead"
order: 67
part: Retrospective
description: "A look back across everything — from a kprobe counting unlink in Chapter 7 to a single request traced through two services and the kernel in Chapter 63. What stayed constant, what changed in how you think about the kernel, the true state of the code, and where eBPF and Aya are heading: the all-Rust frontier, sched_ext and struct_ops, cpufreq_ext, OBI, and the operating layers above."
duration: 25 minutes
---

Sixty-seven chapters ago, the kernel was a closed box you could only watch from
the outside, through whatever counters it chose to expose. Now it's something you
*program* — you pick an event, write a handler in Rust, prove it safe, attach it,
and watch the result flow into Grafana next to a distributed trace. That shift,
from spectator to participant, is the real subject of this book. This chapter
looks back at how the parts built on each other, what held constant the whole
way, the real state of what you have, and where eBPF and Aya go from here.

{% include excalidraw.html
   file="book-arc"
   alt="From a kprobe counting unlink to operating a fleet, and the road ahead. The arc runs through six phases: Foundations (Part 0); Tracing and probing (Parts 1-2); Performance, networking, and security (Parts 3-5); Schedulers through advanced kernel surface (Parts 6-8); Operating eBPF (Part 9); and the Field guide (Part 10). It points to the frontier: all-Rust kernel side, sched_ext, struct_ops, cpufreq_ext, OBI, bpfman and L3AF, and DPU offload. The loop never changed: pick an event, write the handler, attach, observe — Rust to ship, bpftrace, bcc, and bpftool to explore."
   caption="Figure 67.1 — the arc of the book, and where it points next" %}

## How the parts built on each other

The **Foundations** (Part 0) were deliberately front-loaded with a real lab and
a real observability stack, so that the very first program in Chapter 6 reported
into Grafana rather than printing to a terminal. Everything after inherited that:
no program in this book is a toy that only prints.

From there the progression was a widening of *where* you attach. **Tracing the
kernel** (Part 1) started at the most basic dynamic-tracing point — a kprobe on
`unlink` counting file deletions — and worked through `fentry`, tracepoints, and
the `*snoop` family. **User-space & language probing** (Part 2) crossed the
boundary into processes with uprobes, USDT, and the TLS-reading `sslsniff`, then
into language runtimes. **Performance & resources** (Part 3) turned attachment
into measurement: run-queue latency, IRQ timing, CPU profiling, memory leaks.
**Networking** (Part 4) went down the stack to XDP and TC, building real packet
processors — a dropper, a load balancer, a capturer. **Security & LSM** (Part 5)
crossed the last line, from *observing* to *deciding*: LSM hooks that can deny an
operation, not just record it.

The back half raised the ceiling. **Schedulers** (Part 6) and the **advanced
kernel surface** (Part 8) reached the frontier where eBPF implements kernel
interfaces — `sched_ext`, `struct_ops`, iterators, arenas — and where, in 2026,
the newest surface still speaks C first and Aya is catching up. **Application
targets** (Part 7) grounded all of it in the Quarkus and FastAPI services you'd
actually observe. **Operating eBPF** (Part 9) was the shift from "it works on my
VM" to running it for real: CO-RE portability, zero-downtime upgrades, offload,
power attribution, and the signal-correlation work that culminated in the
**Chapter 63 capstone** — one request, traced through two services and the
kernel, on a single `trace_id`. The optional **Field guide** (Part 10) then
handed you the command-line instruments — `bpftrace`, `bpftool`, the BCC tools —
that had been the cross-check in every chapter, now driven from Python.

## What held constant

Strip away the specifics and the same handful of ideas carried the whole way:

- **The loop never changed.** Every chapter was the same four moves: pick an
  event, write the handler, attach it, observe the result. The kprobe in Chapter
  7 and the socket observer in the capstone are the same shape at different scale.
- **Rust earned its place at the verifier.** The bounds-checked slices and
  absence of uninitialized memory that make Rust *Rust* are exactly what the
  kernel **verifier** demands — so a large class of "verifier rejected your
  program" errors simply never happened. That was the thesis from Chapter 5, and
  it held.
- **Maps were the only channel.** From the first counter to the capstone's
  per-request metrics, kernel and user space met only through maps and
  ring/perf output. Get the `#[repr(C)]` layout right and everything worked; get
  it wrong and you read garbage. No exceptions appeared.
- **The lab model protected you.** eBPF ran in the guest VM, never the laptop
  kernel — so a misbehaving program cost you a `virsh destroy`, not your session.
  Building on the host and shipping one binary to the guest was the same motion
  every time.
- **Observability was the default, not an afterthought.** Because the stack was
  there from Chapter 3, every program produced an `ebpf_*` signal you could graph
  — and Part 9 showed that signal joining application traces and logs on one
  `trace_id`. The three-signal story was the spine.
- **Explore with the tools, ship with Aya.** `bpftrace` and the BCC tools found
  the signal in seconds; Aya turned it into a typed, embeddable, single binary
  you could operate for months. The field guide made that division explicit, but
  it was the working pattern all along.

## What changed in how you think

The mechanical skills matter, but the durable change is in how the kernel looks
to you now. It is no longer a fixed surface you query; it's a programmable one
you extend, safely, at runtime, without a reboot. A performance question stops
being "which existing counter is closest?" and becomes "what would I measure if
I could put a probe anywhere?" — and then you put one there. The verifier stops
reading as an obstacle and starts reading as a reviewer that guarantees you can't
crash production. And observability stops being something bolted on at the end
and becomes the natural output of every probe you write.

## The real state of what you have

Every chapter ended the same way, and it matters here most: the code is
**`unverified`**. It was written against the documented behavior of Aya, the
kernel, and the toolchain, but it was not compiled or run on hardware during
authoring — there was no Fedora 44 machine in the loop. That is not a hedge to
apologize for; it's the design. The lab you built in Part 0 is the proving
ground, and *you* are the one who promotes a claim from `unverified` to verified
by running it on a real Fedora 44 kernel and seeing the metric move. Treat every
example as a well-researched starting point that expects your `cargo build` and
your `gh run watch`, not as a guarantee. Where a chapter flagged a specific risk
— a tracepoint field offset, a UBI image tag, an Aya API that may have shifted —
that's where to look first when something doesn't attach.

## Where eBPF and Aya go next

The frontier this book kept reaching (Parts 6, 8, and 9) is moving fast, and the
direction is clear even where the destination isn't:

- **The all-Rust kernel side is maturing.** Today the newest surface —
  `sched_ext` schedulers, `struct_ops` providers, iterators, arenas — is often
  *canonical in C* with Aya as the observer or loader, exactly as those chapters
  flagged. The trajectory is toward authoring all of it in Rust; each Aya release
  closes more of that gap. The goal stated in Chapter 4 — write the whole thing
  in one language — gets closer every cycle.
- **eBPF moves from observing to controlling.** `sched_ext` already lets you
  write a CPU scheduler; **`cpufreq_ext`** (Chapter 61) is bringing frequency
  policy into eBPF; LSM (Part 5) already decides. The next decade of eBPF is as
  much about *steering* the system as watching it.
- **Zero-code instrumentation goes mainstream.** **OBI** (OpenTelemetry eBPF
  Instrumentation, Chapter 46) and its kin produce RED metrics and traces for
  services without touching their code — the correlation story of Part 9, made
  automatic. Expect this to be the default way services get traced.
- **Operating layers grow up.** **bpfman** and **L3AF** (Chapter 59) are turning
  "load a program" into "manage a fleet of programs" — lifecycle, chaining,
  Kubernetes-native deployment. As eBPF becomes infrastructure, these become as
  important as the programs themselves.
- **The hardware boundary keeps moving.** XDP offload, SmartNICs, and DPUs
  (Chapter 60) push eBPF off the host CPU entirely; a DPU running ordinary Aya
  cross-compiled to `aarch64` is already plausible. Where the program runs is
  becoming another axis you choose.

None of this changes the loop. A `sched_ext` scheduler, an OBI-instrumented
service, a program managed by bpfman — each is still an event, a handler, an
attach, and an observation. You already know the shape.

## A closing word

You started this book watching the kernel and finished it programming it, with a
lab to prove your work and a dashboard to show it. The most useful thing now is
not to read another chapter — it's to open the lab, pick a real question about a
system you care about, and write the probe that answers it. The loop is yours.
Build something, run it on real hardware, and let the metric move.

---

*Verification status: this retrospective is reflection, not code — but it points
back at sixty-seven chapters whose claims are <span class="status
status--unverified">unverified</span> until your own run on Fedora 44 promotes
them. That run is the point. Thank you for building this with me.*
