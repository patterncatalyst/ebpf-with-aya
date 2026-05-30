---
title: "uprobe on a Rust binary"
order: 14
part: User-space & language probing
description: Point a uprobe at a function in a Rust program you built — confront symbol mangling and calling conventions, read a function argument at entry, and lay the groundwork for tracing your own services.
duration: 25 minutes
---

Chapter 13 probed someone else's binary (bash) at a symbol that
happened to be C. This chapter probes a **Rust** binary you built, and
reads a function's argument as it's called. Two Rust-specific things
surface immediately — **symbol mangling** and **calling convention** —
and handling them is the point of the chapter. This is also the
foundation for tracing your *own* Rust services later.

The code is in `examples/14-uprobe-rust/`, which includes a small
`target-app` to probe. `./demo.sh` there builds, deploys, and runs it;
its `README.md` covers what it does and how to drive it.

## The two Rust problems

**Mangling.** Rust mangles symbol names (so `module::compute` becomes
something like `_ZN6module7compute17h…E`). A uprobe attaches to a
symbol *by name*, so a mangled name is both hard to type and
version-unstable. The clean fix for code you control is to export the
function unmangled:

```rust
#[no_mangle]
#[inline(never)]
pub extern "C" fn compute(x: u64) -> u64 { /* ... */ }
```

`#[no_mangle]` keeps the symbol literally `compute`. `#[inline(never)]`
guarantees there's a real call site to attach to (an inlined function
has no entry point). And `extern "C"` brings us to the second problem.

**Calling convention.** `ctx.arg(0)` in a uprobe reads the first
argument *register* per the platform C ABI (on x86-64, `rdi`). Rust's
default ABI is unspecified and may pass arguments differently, so for a
function you intend to probe by argument, `extern "C"` pins it to the C
convention and makes `ctx.arg(0)` correct. For probing arbitrary
*existing* Rust code you don't control, you'd instead work from the
mangled symbol and the actual register placement — more involved, and a
good reason to add `#[no_mangle] extern "C"` shims at points you want to
observe.

> For probing third-party Rust binaries you can't modify, you resolve
> the mangled symbol (e.g. with `nm`/`rustfilt`) and attach to it by
> name or offset; argument reading then follows Rust's ABI for that
> signature. This chapter takes the tractable path — your own code, a
> `no_mangle extern "C"` entry point — so the mechanics are clean.

## The target and the probe

`target-app` loops, calling `compute(i)` every half second with an
incrementing `i`. The uprobe reads that argument:

```rust
#[uprobe]
pub fn compute_enter(ctx: ProbeContext) -> u32 {
    let arg0: u64 = ctx.arg(0).ok_or(0)?;     // first C-ABI argument
    // emit ArgEvent { pid, arg0 } to the ring buffer
    0
}
```

Attaching points at the **target binary's path** and the symbol:

```rust
prog.attach(Some("compute"), 0, "/home/fedora/target-app", None)?;
```

Same `attach` shape as Chapter 13 — only the artifact and symbol
change. The uprobe fires inside `target-app`'s process every time
`compute` is entered, reads `rdi`, and ships it to user space.

## Build, deploy, observe

The demo builds both the snoop tool and `target-app`, ships the app to
the VM and starts it in the background, then attaches the uprobe:

```bash
cd examples/14-uprobe-rust && ./demo.sh
```

You'll see the argument climb in lockstep with the app's loop:

```text
PID      compute(arg0)
12345    compute(0)
12345    compute(1)
12345    compute(2)
```

and `ebpf_events_total{program="uprobe-rust"}` rising in Grafana. You
are reading a live argument out of a running Rust program from a probe
in the kernel — without modifying or recompiling the app at attach
time.

## Cross-check

```bash
[vm]$ objdump -T /home/fedora/target-app | grep -w compute
[vm]$ sudo bpftrace -e 'uprobe:/home/fedora/target-app:compute { printf("arg0=%d\n", arg0); }'
```

The `objdump` line proves the symbol is present and unmangled; the
`bpftrace` one-liner reads the same argument independently. If its
`arg0` matches your table, your symbol resolution and argument register
are both right.

> **If you see no events**, the most likely cause is that `--release`
> LTO inlined `compute` away despite `#[inline(never)]`, leaving no call
> site. Confirm with the `objdump` check; if the symbol's gone, build
> `target-app` without LTO. This is the uprobe-on-optimized-code
> reality: aggressive optimization can erase the very functions you
> wanted to watch.

## What you learned

- Probing Rust means handling **mangling** (`#[no_mangle]`) and
  **calling convention** (`extern "C"` so `ctx.arg(0)` is the C-ABI
  first argument), plus keeping a real call site (`#[inline(never)]`).
- You can read live arguments out of your own running binaries — the
  basis for service tracing in later chapters.

Next, the part continues with USDT/BTF-assisted user probes and runtime
tracing (`sslsniff`, `funclatency`, language runtimes).

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm `ProbeContext::arg(0)` for uprobes, the `attach` signature, and
that `#[no_mangle] #[inline(never)] extern "C"` survives `--release`+LTO
with an attachable `compute` symbol. The first build and run are the
test.*
