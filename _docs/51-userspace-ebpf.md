---
title: "Userspace eBPF: the same bytecode, no kernel"
order: 51
part: Advanced kernel surface
description: "eBPF is a portable bytecode ISA, and it doesn't need the kernel. Run an eBPF program in a user-space VM with the Rust rbpf crate — interpreter and JIT, with a Rust helper the bytecode calls — needing no root, no kernel, not even the lab VM. Then meet bpftime, the runtime that accelerates uprobes in user space."
duration: 35 minutes
---

Every chapter so far has loaded BPF into the kernel. But eBPF is, underneath,
just a small **bytecode ISA** — eleven registers, a fixed instruction set, a
helper-call convention — and nothing about that ISA requires the kernel to run
it. There are mature **user-space eBPF runtimes** that execute the exact same
bytecode inside an ordinary process: no `bpf()` syscall, no verifier ceiling,
no root. This chapter steps outside Aya and the kernel entirely to show that
side of the ecosystem — and the example runs on your laptop with `cargo run`,
no lab VM at all, which is the point.

The code is in `examples/51-userspace-ebpf/`, and unlike every other example
it targets the **host**. Its `README.md` has the details.

{% include excalidraw.html
   file="userspace-ebpf"
   alt="The same eBPF bytecode (a portable ISA) runs two ways. Loaded via bpf() into the kernel runtime — verifier plus JIT, kernel hooks, privileged and system-wide. Or embedded in a user-space VM (rbpf, uBPF, bpftime) running in your app, with no root, portable. rbpf is a Rust VM you embed; bpftime runs uprobes and USDT in user space and can share maps with the kernel."
   caption="Figure 51.1 — eBPF is an ISA: the same bytecode runs in the kernel or in a user-space VM" %}

## Why run eBPF outside the kernel

Three reasons people reach for a user-space runtime:

- **No privileges, no kernel ceiling.** Loading into the kernel needs
  `CAP_BPF`/root and must pass the kernel verifier. A user-space VM runs in an
  unprivileged process and has only a tiny verifier of its own — handy for
  embedding programmable logic where you can't or don't want kernel access.
- **Portability.** The kernel runtime is Linux-only. User-space VMs run on
  macOS, Windows, embedded targets, and edge devices — anywhere the host
  binary runs. (Solana even uses a fork of one to execute on-chain programs.)
- **Embedding and performance.** A user-space dataplane (Open vSwitch, DPDK)
  embeds a VM to run packet filters in-process; and **bpftime** runs
  uprobe/USDT handlers in user space to skip the kernel context-switch that
  makes kernel uprobes costly — while still sharing maps with kernel eBPF.

The runtimes you'll meet: **uBPF** (the original, in C), **rbpf** (a Rust port
— interpreter, x86-64 JIT, assembler, disassembler), and **bpftime** (the
modern runtime focused on fast user-space uprobes/USDT with kernel-shared
maps).

## Running eBPF in Rust with rbpf

`rbpf` is a crate you add to any Rust program. You hand it eBPF bytecode,
register helper functions it can call, and execute — interpreted or
JIT-compiled. Here the program loads the first byte of a memory buffer, calls a
helper, and returns the result:

```rust
use rbpf::assembler::assemble;

// a Rust helper the bytecode can call (helper key 1)
fn double(arg: u64, _: u64, _: u64, _: u64, _: u64) -> u64 { arg * 2 }

let prog = assemble("
    ldxb r1, [r1+0]   ; r1 = first byte of the memory buffer
    call 1            ; r0 = double(r1)
    exit              ; return r0
")?;

let mut mem = [21u8, 0, 0, 0];
let mut vm = rbpf::EbpfVmRaw::new(Some(&prog))?;
vm.register_helper(1, double)?;
let result = vm.execute_program(&mut mem)?;   // interpreter -> 42
```

A few things worth noticing:

- The program is the **same eBPF instructions** the kernel would run; `rbpf`
  ships an **assembler** so you can write them as text instead of raw bytes.
- **`register_helper`** is the user-space mirror of the kernel's helper table:
  the bytecode calls helper `1`, and *you* decide what that does — an ordinary
  Rust function. In the kernel that slot would be `bpf_map_lookup_elem` or
  friends; here it's whatever your application provides.
- **`execute_program`** runs the interpreter. Call **`jit_compile()`** and then
  `execute_program_jit()` and the same program runs as native x86-64 — same
  result, faster path. There's no `bpf()` syscall, no root, nothing
  system-wide; it all happens inside this process.

The flip side, stated plainly: rbpf's verifier is a toy compared to the
kernel's, so a user-space VM **trusts the bytecode** far more than the kernel
does. You gain portability and simplicity; you give up the kernel's safety
proof.

## bpftime: user-space speed for kernel-style probes

The most active project here is **bpftime**. It runs **uprobe and USDT**
handlers in user space — attaching via a preloaded library rather than the
kernel — which avoids the trap into the kernel and back that makes kernel
uprobes expensive, reportedly an order of magnitude faster for
uprobe-heavy workloads. Crucially it can **share maps with kernel eBPF**, so a
user-space handler and a kernel program can cooperate. It runs existing eBPF
built with libbpf or Aya, which is the bridge back to everything in this book:
the program you wrote for the kernel can, in many cases, run in bpftime
instead.

## Build, deploy, observe

```bash
cd examples/51-userspace-ebpf && ./demo.sh
```

This one runs **on your host** — no target VM, no root. It assembles the
program, registers the Rust helper, and runs it both interpreted and
JIT-compiled, printing the result each way (both `42`). The takeaway is the
absence of ceremony: eBPF executed, and the kernel was never involved. There's
no Grafana panel here precisely because nothing touched the kernel or the OTLP
stack.

## Cross-check

```bash
# disassemble the bytecode rbpf assembled, to confirm it's real eBPF
[host]$ cargo run --quiet -- --disasm
# the interpreter and the JIT must agree
[host]$ cargo run --quiet        # prints: interpreter=42 jit=42
```

Interpreter and JIT producing the identical result on the same bytecode is the
check that the VM is faithful; disassembling shows the instructions are the
ordinary eBPF you'd recognize from any kernel program.

## What you learned

- eBPF is a **portable bytecode ISA**; user-space runtimes (**uBPF**, **rbpf**,
  **bpftime**) execute the same instructions with no kernel, no root, and
  cross-platform.
- With **rbpf** you embed an eBPF VM in a Rust program, register Rust
  **helpers** the bytecode calls, and run it interpreted or JIT-compiled — at
  the cost of the kernel verifier's safety guarantees.
- **bpftime** is the runtime to know: user-space uprobes/USDT that skip the
  kernel context switch and can share maps with kernel eBPF, running programs
  built with libbpf or Aya.

Next, Chapter 52 returns to the kernel for **kfuncs** — the modern, typed
alternative to the fixed helper list these VMs reimplement.

---

*Verification status: <span class="status status--verified">verified</span>
— host run (no VM, no root). `cargo run --release` builds against the current
`rbpf` API (`assembler::assemble`, `EbpfVmRaw::new`, `register_helper`,
`execute_program`, `jit_compile` / `execute_program_jit`) and the interpreter
and JIT agree: `interpreter=42 jit=42`. The JIT path is x86-64 only.*
