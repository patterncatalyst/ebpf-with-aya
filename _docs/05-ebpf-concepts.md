---
title: "eBPF concepts and tools"
order: 5
part: Foundations
description: Programs, maps, the verifier, BTF and CO-RE, program types and attach points — the mental model you need before writing the first Aya program, and how bpftool and bpftrace fit in.
duration: 20 minutes
---

You have a lab and a toolchain. Before writing the first program, this
chapter builds the mental model: what an eBPF program actually *is*,
how it gets from your `.rs` file into a running kernel, what the
verifier will and won't let you do, and how Aya's pieces map onto the
kernel's. It's deliberately concept-only — no deploys — so that when
Chapter 6's code appears, every line has a place to land.

{% include excalidraw.html
   file="ebpf-lifecycle"
   alt="eBPF program lifecycle: Rust source compiles to BPF bytecode, is loaded and verified, JIT-compiled to native code, and attached to a hook (kprobe, tracepoint, uprobe, perf_event, XDP/tc/LSM) that fires it."
   caption="Figure 5.1 — load, verify, JIT, attach" %}

## What an eBPF program is

eBPF lets you load a small program into the running kernel and attach
it to an **event** — a function entry, a syscall, a packet arriving on
a NIC, a tracepoint firing. When the event happens, your program runs,
in kernel context, and then returns. It cannot loop forever, cannot
call arbitrary kernel functions, and cannot touch arbitrary memory.
Those restrictions are the price of safety, and they're enforced
before your program ever runs.

Think of it as the kernel's plugin system: you don't recompile or
reboot the kernel, you hand it a verified program and it runs that
program at the events you chose.

## The load-and-attach lifecycle

Every Aya program follows the same arc, and it's worth memorizing
because the API mirrors it exactly. Figure 5.1 showed the *build* side —
source to bytecode to a verified, JIT-compiled program attached to a hook.
Figure 5.2 shows what happens once it's running: the hook fires the program,
the program writes a map, and your loader reads that map and reports it.

{% include excalidraw.html
   file="ebpf-runtime-loop"
   alt="At runtime, the loader binary in user space loads and attaches the program once. Then in the kernel a hook (kprobe, XDP, tracepoint) fires the eBPF program, which writes to a map (RingBuf or HashMap); the loader reads that map and reports to Grafana over OTLP. The map is the only channel between kernel and user space."
   caption="Figure 5.2 — at runtime: the hook fires the program, which writes a map the loader reads" %}

In Aya, "load" is `Ebpf::load(...)`, "attach" is `program.load()`
followed by `program.attach(...)`, and "read the map" is opening a
typed map handle and iterating it. Chapter 6 shows each of these.

## Maps: how the two worlds share data

An eBPF program has no return channel to user space except **maps**.
A map is a kernel-resident data structure both sides can access by a
file descriptor. The kinds you'll use most early:

- **`PerCpuArray` / `Array`** — fixed-size; great for counters.
  Per-CPU variants avoid cross-CPU contention; user space sums them.
- **`HashMap`** — keyed lookups, e.g. "latency histogram bucket per
  PID" or "first-seen timestamp per socket".
- **`PerfEventArray` / `RingBuf`** — streaming events from kernel to
  user space. `RingBuf` (the modern choice) is a single shared ring;
  perfect for "emit one record per `execve`".

The shared `*-common` crate (Chapter 4) holds the `#[repr(C)]` structs
that go *into* maps, so the kernel writer and the user-space reader
agree on the byte layout. Get that layout wrong and you'll read
garbage — it's the most common early bug.

## The verifier: your strict but fair reviewer

Before your program runs, the kernel **verifier** walks every possible
execution path and proves the program is safe: it terminates (bounded
loops only), never dereferences an unchecked pointer, never reads
uninitialized stack, and stays within its instruction budget. If it
can't prove safety, it rejects the load with an error.

Two things to know now:

1. **Verifier errors are normal**, especially early. They read like a
   disassembly dump with a complaint at the end. The usual causes are
   an unchecked bounds access (you indexed a packet without first
   checking the packet is long enough) or an unbounded loop.
2. **Rust + Aya prevents whole classes of these at compile time.**
   Bounds-checked slice access, no uninitialized memory, no wild
   pointers — the same guarantees that make Rust Rust also keep the
   verifier happy. That's a large part of *why* write eBPF in Rust.

## Anatomy: how bytecode becomes a running probe

It's worth seeing the whole subsystem at once, because every later chapter is a
variation on this one picture. Your Rust compiles (via `bpf-linker`, Chapter 4)
to **BPF bytecode**; the loader hands that bytecode to the kernel with the
`bpf()` syscall; the **verifier** proves it safe; the kernel **JIT-compiles** it
to native instructions and attaches it to an **event source** — a kprobe, a
uprobe, a tracepoint, a perf event. From then on the program runs on every event
and shares data with user space through exactly two channels: **maps** (lookup
tables both sides hold by file descriptor) and **perf/ring-buffer output**
(streamed records). There is no other way in or out — which is precisely what
makes eBPF safe to run in the kernel.

{% include excalidraw.html
   file="bpf-architecture"
   alt="The BPF subsystem. In user space you generate BPF bytecode and (2) load it into the kernel, where the verifier checks it and hands it to the BPF runtime; the BPF program attaches to event sources — kprobes, uprobes, tracepoints, perf_events — and reads and writes maps. Data returns to user space two ways: perf_output streams per-event data, and the loader does async reads of map contents for statistics. Maps and perf/ring output are the only channels between kernel and user space."
   caption="Figure 5.3 — generate, load, verify, attach; talk to user space only via maps + perf/ring output" %}

The dashed line down the middle of the figure is the boundary that matters:
**user space** on the left, the **kernel** on the right. Crossing it safely is
the entire point of eBPF. Read the pipeline as seven steps — the first three are
the ①②③ marked on the diagram:

1. **Compile (①).** `cargo build` runs your eBPF crate through `rustc` and
   `bpf-linker` (Chapter 4) into **BPF bytecode**, embedded into the loader
   binary with `include_bytes_aligned!`.
2. **Load (②).** The loader calls `Ebpf::load(...)`, handing that bytecode to
   the kernel through the `bpf()` syscall — the first crossing into kernel space.
3. **Verify.** The kernel **verifier** walks every path and proves the program
   safe; if it can't, the load returns an error and nothing runs.
4. **JIT-compile.** Accepted bytecode is translated to native machine code, so
   it runs at full speed rather than being interpreted.
5. **Attach.** `program.attach(...)` wires the program to an **event source** —
   a kprobe, uprobe, tracepoint, or perf event (and later XDP, tc, LSM).
6. **Run.** On every event the program executes in kernel context and writes
   results into a **map**, or streams a record out a **ring/perf buffer**.
7. **Consume (③).** Back in user space, your loader reads the map (an *async
   read*) or drains the ring buffer (*perf_output*) and does something with the
   data — in this book, exports it over OTLP to Grafana.

That final step is where "user space" earns its name, and it needn't be Rust:
the Part 10 field-guide tools (`bpftrace`, `bcc`) follow the *exact same* kernel
path — compile, load, verify, attach, run — but are driven and read from
**Python** instead of an Aya loader. Same pipeline, different front end.

This is the same arc as Figures 5.1 and 5.2, drawn as a system rather than a
timeline. Aya gives each kernel-side piece a typed Rust name — `Ebpf::load` for
step 2, `program.attach` for step 5, a typed map handle for step 7 — so the code
you write maps one-to-one onto this diagram.

## BTF and CO-RE: compile once, run everywhere

Kernel data structures (`struct task_struct`, `struct file`, …) change
layout between kernel versions. A probe that reads a field at a
hardcoded offset breaks the moment the kernel changes. **BTF** (BPF
Type Format) is the kernel describing its own types to you at runtime —
that `vmlinux` blob you confirmed at `/sys/kernel/btf/vmlinux` in
Chapter 2. **CO-RE** (Compile Once – Run Everywhere) uses BTF to
*relocate* field accesses at load time, so one compiled program runs
across kernel versions without recompiling.

Aya supports CO-RE transparently when the target kernel has BTF —
which Fedora 44's stock kernel does. This is why you can build on your
laptop and run on the guest even if their kernels differ slightly, and
why we confirmed BTF presence during lab setup.

## Program types and attach points

The *type* of an eBPF program determines what it can do and where it
attaches. The ones this tutorial works through, grouped by part:

| Family | Attaches to | Tutorial chapters |
|--------|-------------|-------------------|
| **kprobe / kretprobe** | any kernel function entry/return | `kprobe+unlink`, `opensnoop` |
| **fentry / fexit** | function entry/exit (BTF-based, lower overhead than kprobes) | `fentry+unlink` |
| **tracepoint / raw tracepoint** | stable kernel trace events | `execsnoop`, `exitsnoop`, `sigsnoop` |
| **uprobe / USDT** | user-space function entry, in any process | `bashreadline`, `uprobe rust`, `sslsniff` |
| **perf / profiling** | sampled on a timer or PMU counter | `profile`, `runqlat`, `hardirqs` |
| **XDP** | earliest point a packet hits the NIC driver | `xdp`, load balancer, `xdp tcpdump` |
| **TC / tcx** | traffic control ingress/egress | `tc`, `tcx` |
| **socket / sockops** | socket lifecycle and data | `L7 tracing`, `sockops` |
| **LSM** | security hooks; can *deny* operations | `lsm connect`, hardening |
| **struct_ops / sched_ext** | implement a kernel interface in BPF | `scx_simple`, `scx_nest` |

You don't need to memorize this. The point is that "write an eBPF
program" always means "pick a program type, write the handler, attach
it to the right event" — and Aya gives you a typed Rust macro for each
type (`#[kprobe]`, `#[xdp]`, `#[tracepoint]`, …).

### The wider event landscape

Those families fit into a larger map of *everything* eBPF can hook, and it's
worth seeing once so the scope is clear. eBPF attaches across the entire stack:
**dynamic tracing** of any function (kprobes in the kernel, uprobes in user
space), **static tracepoints** the kernel exposes at stable points
(`syscalls:`, `sched:`, `block:`, `net:`, …), hardware **PMCs** (cycles,
instructions, cache misses) and **software events** (page faults, context
switches) sampled through the perf subsystem. Each capability arrived in a
specific kernel (kprobes in 4.1, uprobes in 4.3, the syscall tracepoints in 4.7,
PMC/software-event access in 4.9) — all comfortably present in Fedora 44's modern 6.x kernel — the one this
workshop runs on — which is why the tutorial can reach any of them.

{% include excalidraw.html
   file="linux-events-bpf"
   alt="Linux events and BPF support. Around the kernel stack — applications, system libraries, the system call interface, VFS/sockets/scheduler, file systems/TCP-UDP/virtual memory, block/net devices, device drivers — eBPF can attach via dynamic tracing (uprobes for user space, kprobes for the kernel, spanning the whole stack), static tracepoints (syscalls, sched, signal, timer, block, net, skb, irq, ext4, sock, kmem, vmscan, writeback, workqueue), hardware PMCs on the CPU (cycles, instructions, branch and cache counters, with a memory bus to DRAM), and software events (cpu-clock, context switches, migrations, page faults). Each became available in a specific kernel version."
   caption="Figure 5.4 — what eBPF can observe: dynamic tracing, tracepoints, PMCs, and software events across the stack" %}

The practical takeaway is reach: if the kernel or a user program *does*
something, there is almost certainly an eBPF attach point for it — and the rest
of the book is a guided tour of the most useful ones.

## Where bpftool and bpftrace fit

Two Fedora-packaged tools are your ground truth when an Aya program
misbehaves. You run these **inside the target VM**:

| Tool | What it is | What it's for | Try |
|---|---|---|---|
| `bpftool` | inspects the live BPF subsystem | what's loaded, what maps hold, and what's attached where — when user space reads zeros from a map, it tells you whether the *kernel* side is writing anything at all, isolating the bug to one half | `bpftool prog list` · `bpftool map dump` |
| `bpftrace` | a high-level tracing language (not Aya) | the fastest way to confirm an event even fires before you invest in a full program | `bpftrace -e 'tracepoint:syscalls:sys_enter_openat { @[comm]=count() }'` |

Treat them as the multimeter you check against, not as competitors to
Aya. The tutorial uses them throughout to verify that what your Rust
program reports matches what the kernel actually did — and **Part 10**
returns to both, plus the BCC tools, to show how to drive them from Python.

## The shape of every chapter from here

With this model, each program chapter follows the same shape:

1. **Concept** — which event, which program type, and what we'll measure,
   with a diagram of the flow.
2. **How the code works** — the maps (and why each type), the kernel handler
   in full, and the user-space side (load, attach, drain the map, export via
   OTLP); plus the shared record type, when the program has one.
3. **Build, deploy, observe** — `demo.sh` ships the binary to the target,
   drives load (often from a Python client), and shows the result in Grafana.
4. **Cross-check** — confirm against `bpftool`, `bpftrace`, or the native
   tool the program imitates.
5. **What you learned** — a short recap.
6. **Verification status** — every claim starts `unverified` and stays that
   way until your own run on real hardware promotes it.

[Next: Chapter 6 — Hello, eBPF →]({{ "/docs/06-hello-world/" | relative_url }})

---

*This chapter is conceptual; its claims about Aya's API surface and
kernel behavior are <span class="status status--unverified">unverified</span>
against a running system until Chapter 6's program is built and
deployed. The concepts themselves draw on* Learning eBPF *(Liz Rice),*
BPF Performance Tools *(Brendan Gregg), the Aya Book, and
[ebpf.io](https://ebpf.io/get-started).*
