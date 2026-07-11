---
title: "Writing a scheduler: sched_ext and struct_ops"
order: 43
part: Schedulers (sched_ext)
description: "A new kind of BPF program: implementing a CPU scheduler with sched_ext and the struct_ops model. Walk the minimal scx_simple scheduler's callbacks, understand dispatch queues and the safety watchdog, then run the real scheduler and watch it work with an Aya tracing probe."
duration: 50 minutes
---

Until now every BPF program in this book has *hooked* the kernel — observed
an event or mediated a decision. `sched_ext` is different: it lets a BPF
program *be* a kernel subsystem. With it you implement the CPU scheduler —
the code that decides which thread runs on which core, and for how long — as
a set of BPF callbacks, and load it into a running kernel without a reboot.
This is built on **`struct_ops`**, a BPF model where your program provides a
*structure of function pointers* the kernel calls, and it's the foundation
of the most modern BPF surface. This chapter teaches that model through the
smallest real scheduler, `scx_simple`, and is candid about where the Rust
tooling currently stands.

The code and runbook are in `examples/43-scx-simple/`. `./demo.sh` there
loads the real `scx_simple`, runs a workload, and attaches a small Aya probe
to watch it; its `README.md` covers the details.

{% include excalidraw.html
   file="scx-simple"
   alt="A runnable task enters your BPF scheduler, which implements sched_ext_ops via struct_ops with callbacks select_cpu, enqueue, and dispatch. The scheduler enqueues the task onto a dispatch queue (DSQ), and the kernel dispatches it from there to a CPU to run. You implement the callbacks; the kernel calls them to place and run every task. A misbehaving scheduler is auto-evicted by the watchdog, and the kernel falls back to the default scheduler."
   caption="Figure 43.1 — You implement the callbacks; the kernel places and runs tasks through them" %}

> **A note on language, up front.** Everywhere else this book writes the BPF
> side in Rust with Aya. `sched_ext` is the one place that isn't yet the
> common path: today schedulers are written with the **BPF callbacks in C**
> (the upstream [`scx`](https://github.com/sched-ext/scx) project) and the
> **user-space half in Rust** (the `scx_utils` / `libbpf` crates). Aya's
> kernel-side `struct_ops` support is still maturing. So this chapter does
> the candid thing: it teaches the model with `scx_simple`'s real C
> callbacks, runs the actual shipping scheduler, and uses your Aya tracing
> skills to observe it — rather than pretending a from-scratch Aya scheduler
> is a turnkey thing today.

## Requirements

`sched_ext` needs **kernel 6.12 or newer** with `CONFIG_SCHED_CLASS_EXT=y`
(Fedora 44 has both). Check on the target:

```bash
[vm]$ ls /sys/kernel/sched_ext/         # present means sched_ext is available
[vm]$ cat /sys/kernel/sched_ext/state   # "disabled" until a scheduler loads
```

The example installs Fedora's `scx-scheds` package, which ships `scx_simple`
and friends as ready-to-run binaries.

## The struct_ops model

Most BPF programs are a single function the kernel calls at a hook.
`struct_ops` is the opposite shape: the kernel defines a **struct of callback
slots** (an interface), and your BPF program fills them in. The loader
registers that filled-in struct with the kernel, which then calls your
functions wherever it would have called its own. It's how BPF implements
whole pluggable policies — TCP congestion control uses it too — and
`sched_ext` exports its interface as **`struct sched_ext_ops`**.

The scheduler-relevant callbacks, and what each is for:

- **`select_cpu(p, prev_cpu, wake_flags)`** — when a task wakes, suggest a
  CPU for it (ideally an idle one). A good pick here often lets the task run
  immediately.
- **`enqueue(p, enq_flags)`** — the task is runnable; decide *where to park
  it* until a CPU is free. You place it on a **dispatch queue**.
- **`dispatch(cpu, prev)`** — a CPU needs work; move tasks from a dispatch
  queue onto it. (For simple policies the kernel does this for you from the
  built-in global queue.)
- **`init` / `exit`** — set up and tear down; `exit` also reports *why* the
  scheduler stopped (including the watchdog evicting it).

### Dispatch queues (DSQs)

A **DSQ** is the queue tasks wait on between "runnable" and "running." There's
a built-in global one, `SCX_DSQ_GLOBAL`, and you can create your own
(per-core, per-cgroup, per-priority). The whole art of a scheduler is *which
DSQ a task goes on and in what order CPUs drain them*. The simplest possible
policy — and exactly what `scx_simple` does in FIFO mode — is "put everything
on the global DSQ and let the kernel hand it out."

## Walking scx_simple

Here is the heart of `scx_simple`, the minimal scheduler, in the C the
upstream project uses. Read it as policy, not syntax — the shape is what
matters:

```c
s32 BPF_STRUCT_OPS(simple_select_cpu, struct task_struct *p, s32 prev_cpu, u64 wake_flags)
{
    bool is_idle = false;
    s32 cpu = scx_bpf_select_cpu_dfl(p, prev_cpu, wake_flags, &is_idle);
    if (is_idle)
        scx_bpf_dispatch(p, SCX_DSQ_LOCAL, SCX_SLICE_DFL, 0); /* run right here, now */
    return cpu;
}

void BPF_STRUCT_OPS(simple_enqueue, struct task_struct *p, u64 enq_flags)
{
    /* otherwise: park it on the global queue with a default time slice */
    scx_bpf_dispatch(p, SCX_DSQ_GLOBAL, SCX_SLICE_DFL, enq_flags);
}

s32 BPF_STRUCT_OPS_SLEEPABLE(simple_init) { return 0; }
void BPF_STRUCT_OPS(simple_exit, struct scx_exit_info *ei) { UEI_RECORD(uei, ei); }

SCX_OPS_DEFINE(simple_ops,
    .select_cpu = (void *)simple_select_cpu,
    .enqueue    = (void *)simple_enqueue,
    .init       = (void *)simple_init,
    .exit       = (void *)simple_exit,
    .name       = "simple");
```

Reading it as a scheduler:

- **`select_cpu`** asks the kernel's default idle-CPU picker
  (`scx_bpf_select_cpu_dfl`) for a core. If that core is idle, it dispatches
  the task to the CPU's **local** DSQ with a default slice — meaning "run it
  there immediately." Snapping waking tasks onto idle cores is most of what
  makes a system feel responsive.
- **`enqueue`** is the fallback for tasks that didn't get an idle CPU: park
  them on **`SCX_DSQ_GLOBAL`** with a default time slice. The kernel
  automatically pulls from the global DSQ when a CPU goes looking for work,
  so no explicit `dispatch` callback is needed.
- **`init`/`exit`** are trivial; `exit` records the reason via `UEI_RECORD`
  so user space can print *why* the scheduler stopped — which, crucially,
  includes the watchdog.
- **`SCX_OPS_DEFINE`** is the `struct_ops` declaration: it lays the callbacks
  into a `struct sched_ext_ops` in a special ELF section the loader hands to
  the kernel. That registration *is* "install this scheduler."

That's a complete, working scheduler in ~30 lines: idle-CPU fast path plus a
global FIFO. Everything fancier — weighted fairness, per-core queues,
latency classes — is a richer answer to the same two questions, *which DSQ*
and *in what order*.

## Safety: the watchdog

Handing scheduling to arbitrary code sounds terrifying; `sched_ext` makes it
safe. The verifier still proves your callbacks can't corrupt memory, and on
top of that a **watchdog** monitors that runnable tasks actually get to run.
If your scheduler stalls (a task starves past a timeout), or you trigger the
sysrq escape, the kernel **rips your scheduler out and falls back to the
default** (CFS/EEVDF) without a crash. You can wedge throughput, but you
can't hang the machine — which is what makes experimenting with real
schedulers on a real system reasonable.

## Build, deploy, observe

```bash
cd examples/43-scx-simple && ./demo.sh
```

The demo installs `scx-scheds` if needed, starts `scx_simple` on the target
(so it becomes the active scheduler), runs a short CPU workload, and attaches
a tiny **Aya** probe — a `sched_switch` tracepoint counting context switches
per CPU. **In the terminal** you'll see the per-CPU switch counts tick up as
the workload runs under `scx_simple` (confirm it's active with
`/sys/kernel/sched_ext/state`). **In Grafana** (`127.0.0.1:3000` → Explore),
chart `ebpf_ctxsw_total{cpu}` to watch context switches per core over time.
Stopping `scx_simple` returns the system to the default scheduler.

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`):

```bash
[vm]$ cat /sys/kernel/sched_ext/state            # "enabled" while scx_simple runs
[vm]$ cat /sys/kernel/sched_ext/root/ops         # the active scheduler's name: simple
[vm]$ sudo bpftool struct_ops list               # the registered sched_ext_ops
```

`state` flipping to `enabled` and the ops name reading `simple` confirms your
BPF scheduler is the one running the machine; `bpftool struct_ops list` shows
the registered struct — the `struct_ops` mechanism made visible.

## What you learned

- **`struct_ops`** lets a BPF program implement a kernel interface by filling
  in a struct of callbacks — the model behind `sched_ext` (and TCP
  congestion control).
- A scheduler answers two questions — **which dispatch queue** a task goes on
  and **in what order CPUs drain them**; `scx_simple` is the minimal answer
  (idle-CPU fast path + global FIFO).
- The **watchdog** makes this safe: a broken scheduler is evicted and the
  kernel falls back to the default, so you can experiment on a live system.
- Where the tooling is today: BPF callbacks in C, user space in Rust; Aya's
  kernel-side `struct_ops` is emerging, so we ran the real scheduler and
  observed it with an Aya probe.

Next, Chapter 44 moves past the minimal policy to a more realistic one —
`scx_nest`-style ideas about keeping work on warm cores — to see how dispatch
queues express a real scheduling strategy.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
