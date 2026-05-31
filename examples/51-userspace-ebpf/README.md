# 51 · Userspace eBPF: the same bytecode, no kernel

eBPF is a portable bytecode ISA. This example runs a real eBPF program in a
**user-space VM** (the Rust `rbpf` crate) — interpreter and JIT — with a Rust
helper the bytecode calls. **No kernel, no root, no lab VM**: it runs on your
host with `cargo run`.

## Run it

```bash
./demo.sh           # build, disassemble, run (interpreter + JIT) on the host
./demo.sh build     # just build
cargo run -- --disasm
```

Expected: `interpreter=42 jit=42` (mem[0]=21, the helper doubles it).

## The landscape

- **uBPF** — the original user-space eBPF VM (C); used by Open vSwitch, DPDK.
- **rbpf** — a Rust port (interpreter + x86-64 JIT + assembler); used here.
- **bpftime** — runs uprobe/USDT handlers in user space (skips the kernel
  context switch, ~10x faster for uprobe-heavy work), shares maps with kernel
  eBPF, and runs programs built with libbpf or Aya.

## Verification status

**Unverified.** Confirm the `rbpf` API against the current crate version, that
interpreter and JIT agree, and note the JIT is x86-64 only. Runs on the host;
nothing here touches the kernel or the lab VM.
