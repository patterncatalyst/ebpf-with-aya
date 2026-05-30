---
title: "Tracing goroutine states"
order: 19
part: User-space & language probing
description: "Probe the Go scheduler's state machine through runtime.casgstatus — and meet two Go-specific realities head-on: the register ABI (args aren't where C puts them) and why you must never put a uretprobe on Go."
duration: 30 minutes
---

Go multiplexes thousands of goroutines onto a handful of OS threads, and
its scheduler funnels **every** goroutine state transition through one
function: `runtime.casgstatus`. A uprobe there is a window onto the
scheduler's state machine — runnable, running, waiting, syscall, dead.
Building it forces you to confront two things that make probing Go
different from probing C: the **register ABI** and the **uretprobe
hazard**.

The code is in `examples/19-goroutine-states/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="goroutine-states"
   alt="Goroutine state machine: goroutines move between idle, runnable, running, waiting, and dead; runtime.casgstatus is called on every transition, so one uprobe observes the whole machine."
   caption="Figure 19.1 — the goroutine state machine" %}

## One function sees every transition

`runtime.casgstatus(gp *g, oldval, newval uint32)` is Go's
compare-and-set for goroutine status. The runtime calls it on every
transition, with the goroutine pointer and the old/new states. Probe its
entry, read `newval`, and you can count and chart transitions by state —
a live read on scheduler pressure (lots of `waiting ↔ runnable` churn
means contention or blocking).

## Gotcha 1: Go's register ABI

{% include excalidraw.html
   file="go-vs-c-abi"
   alt="Go register ABI vs C ABI: the C ABI puts arg0/arg1/arg2 in RDI/RSI/RDX, but Go's ABIInternal uses RAX/RBX/RCX, so ctx.arg(2) reads the wrong register and newval must be read from RCX."
   caption="Figure 19.2 — why ctx.arg(n) is wrong for Go (register ABI)" %}

Here's where naive probing breaks. Since Go 1.17, Go uses its **own**
calling convention (ABIInternal), not the platform C ABI. Integer and
pointer arguments go in this register sequence on amd64:

```text
RAX, RBX, RCX, RDI, RSI, R8, R9, R10, R11
```

So for `casgstatus(gp, oldval, newval)`: `gp`→RAX, `oldval`→RBX,
`newval`→**RCX**. But `ctx.arg(2)` in Aya reads the *C ABI's* third
integer register (RDX) — which holds something unrelated. Using
`ctx.arg(n)` on a Go function gives you garbage.

The fix is to read the register Go actually used. We reach into
`pt_regs` and read RCX directly:

```rust
let regs = ctx.as_ptr() as *const pt_regs;
let newstate = unsafe { (*regs).rcx } as u32;   // Go ABI: 3rd int arg
```

This is the general lesson for any non-C-ABI language: `ctx.arg(n)`
assumes the C ABI, so when the target uses a different convention (Go,
sometimes Swift, hand-written assembly), you map arguments to registers
yourself. Confirm the mapping with `bpftrace`'s `reg()` builtin (the
cross-check below).

## Gotcha 2: never put a uretprobe on Go

This one can crash the program you're observing. Go **moves and grows
goroutine stacks** at runtime; a uretprobe works by patching the return
address on the stack to a trampoline, and when Go relocates that stack,
the patched address becomes invalid — corrupting the goroutine, often
fatally. **Use uprobes (entry) only on Go binaries.** If you need a
function's result, probe a different entry point that receives it, or
read it from a later transition — never a uretprobe.

Our program attaches a single uprobe to `casgstatus`. That's deliberate,
not incidental.

## A note on identity: thread vs. goroutine

`bpf_get_current_pid_tgid()` gives the **OS thread (M)** the goroutine is
currently running on — not a goroutine id. Go doesn't expose a stable
goroutine id cheaply, and a goroutine can move between threads. For
counting transitions by state that's fine; if you needed per-goroutine
tracking you'd read the `g` pointer (in RAX) as the identity key. We key
nothing here — we just tally states.

## Build, deploy, observe

This chapter needs the Go toolchain on the host (Fedora repos):

```bash
sudo dnf install -y golang
cd examples/19-goroutine-states && ./demo.sh
```

The demo builds a small Go program that churns goroutines (channels,
sleeps, workers), ships it to the VM, runs it, and attaches the uprobe.
You'll see a stream of `NEW STATE` transitions, and
`ebpf_events_total{program="goroutine",state=…}` in Grafana broken down
by state — the shape of which tells you what the scheduler is spending
its time on.

## Cross-check

```bash
[vm]$ nm /home/fedora/target-go | grep runtime.casgstatus
[vm]$ sudo bpftrace -e 'uprobe:/home/fedora/target-go:runtime.casgstatus { @[reg("cx")] = count(); }'
```

`bpftrace`'s `reg("cx")` reads the same RCX we do — if its per-value
counts match your per-state counts, your register mapping is right. (Go
embeds its symbol table by default, so `nm` finds `runtime.casgstatus`
unless the binary was built with `-ldflags=-s`.)

## What you learned

- One function, `runtime.casgstatus`, exposes Go's whole goroutine state
  machine.
- Non-C languages use different **calling conventions** — read the right
  register (`newval` in RCX for Go), not `ctx.arg(n)`.
- **Never uretprobe Go** — moving stacks make return trampolines unsafe.
- eBPF sees the OS thread, not the goroutine.

Last in this part: **`javagc`**, timing JVM garbage collection through
USDT probes.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Highest-risk: the Go ABI register read (RCX) and the `pt_regs.rcx` field
name in aya 0.13.x; the goroutine-state value mapping for your Go
version. Confirm the register with `bpftrace reg("cx")`. The first build
and run are the test.*
