---
title: "A realistic policy: keeping work on warm cores"
order: 44
part: Schedulers (sched_ext)
description: "Move past the minimal scheduler to a real strategy. The scx_nest policy concentrates work on a small set of warm, high-frequency cores instead of spreading it thin — express it through CPU selection and a nest of cores, run the real scheduler, and watch the concentration appear with an Aya per-CPU busy probe."
duration: 45 minutes
---

`scx_simple` answered the scheduling questions in the dumbest way that works:
everything on one global queue, any idle CPU grabs it. A real policy is an
*opinion* about where work should go. `scx_nest` has a sharp one, borrowed
from the "nest" research: **don't spread threads across all cores — pile them
onto a few warm ones.** Counterintuitive, until you remember CPU frequency
scaling: a core that's been busy runs at turbo with hot caches, while five
lightly-loaded cores all idle down to low frequency. For many workloads,
three busy cores beat eight sleepy ones. This chapter explains that policy,
shows how dispatch-queue and CPU-selection choices express it, and — keeping
the honest pattern from Chapter 43 — runs the real `scx_nest` and makes its
behavior *visible* with an Aya probe.

The code and runbook are in `examples/44-scx-nest/`. `./demo.sh` there runs
the real `scx_nest`, drives a moderate load, and attaches an Aya per-CPU busy
probe; its `README.md` covers the details.

{% include excalidraw.html
   file="scx-nest"
   alt="Tasks flow into the scx_nest policy, which keeps work on warm cores. A primary nest of warm, high-frequency cores (CPU0, CPU1, CPU2) is busy, while a reserve of idle or demoted cores (CPU3, CPU4) stays idle. The scheduler prefers placing tasks in the nest. The idea: concentrate work on a few warm cores with high frequency and hot caches rather than spreading it thin; cores idle too long are demoted out of the nest, and the nest grows under load."
   caption="Figure 44.1 — Concentrate work on a few warm cores instead of spreading it thin" %}

## The nest idea

Spreading N threads across N cores feels fair and is often slower. Each core
sees light load, so the hardware drops its frequency; caches stay cold
because work keeps migrating. The **nest** policy inverts it:

- Keep a **primary nest** — a small set of cores that are kept warm and busy.
  New work prefers an idle core *inside the nest*, so those cores stay at
  high frequency with hot caches.
- If the nest has no idle core, **promote** a core from a **reserve** into
  the nest rather than waking a cold one far away.
- A core that sits **idle too long gets demoted** out of the nest, so the
  nest shrinks back down when load drops.

The result is a nest that breathes with load: small and hot when the system
is lightly loaded, growing only as far as it must. The payoff is real
frequency and cache wins at low-to-moderate utilization — exactly where a
spread-everything scheduler leaves performance on the table.

## How the policy is expressed

The mechanics are the same primitives as `scx_simple` — dispatch queues and
CPU selection — pointed at a different goal. The nest lives in a **cpumask**
(a bitmap of which cores are currently in the nest), kept in a map, and the
work happens in `select_cpu`:

```c
/* Simplified from the real scx_nest — the idea, not the whole thing. */
s32 BPF_STRUCT_OPS(nest_select_cpu, struct task_struct *p, s32 prev_cpu, u64 wake_flags)
{
    /* 1. Prefer an idle core already in the primary nest. */
    s32 cpu = pick_idle_in_mask(&primary_nest);
    if (cpu >= 0) {
        scx_bpf_dispatch(p, SCX_DSQ_LOCAL, SCX_SLICE_DFL, 0); /* run it there now */
        return cpu;
    }
    /* 2. None idle? Promote a reserve core into the nest and use it. */
    cpu = pick_idle_in_mask(&reserve);
    if (cpu >= 0) {
        bpf_cpumask_set_cpu(cpu, &primary_nest);              /* grow the nest */
        scx_bpf_dispatch(p, SCX_DSQ_LOCAL, SCX_SLICE_DFL, 0);
        return cpu;
    }
    /* 3. Everything busy — fall back to the previous core / global queue. */
    return prev_cpu;
}
```

Reading it as policy:

- Step 1 is the whole point: **prefer cores already in the nest**, so work
  lands on hardware that's already warm and clocked up.
- Step 2 is controlled growth: only when the nest is saturated do we
  **promote** a reserve core, expanding the warm set by exactly one.
- A separate path (a timer or the `update_idle` callback, omitted here)
  handles **demotion** — clearing a core's bit from `primary_nest` after it's
  been idle past a threshold, so the nest contracts when load falls.

`bpf_cpumask_*` are the **kfuncs** sched_ext exposes for manipulating core
sets — kernel functions BPF can call directly, the modern alternative to the
fixed helper list (a topic Part 8 returns to). Everything else is the
`scx_simple` vocabulary: `scx_bpf_dispatch`, `SCX_DSQ_LOCAL`, slices. The
*strategy* changed; the *primitives* didn't.

## Making the nest visible

A scheduler's behavior is invisible by default — it either feels fast or it
doesn't. The way to *see* the nest is to measure **per-CPU busy time**: under
`scx_nest` at moderate load, a few cores should sit near fully busy while the
rest stay near idle; under the default scheduler the same work smears evenly
across all cores. That contrast is the policy made observable, and it's
exactly the kind of thing your Aya tracing skills measure.

The example's probe hooks `sched:sched_switch` and, on each switch, attributes
the interval since the last switch on that CPU to "busy" if the outgoing task
wasn't the idle task:

```rust
#[tracepoint] // sched:sched_switch
pub fn on_switch(ctx: TracePointContext) -> u32 {
    let cpu = unsafe { bpf_get_smp_processor_id() };
    let now = unsafe { bpf_ktime_get_ns() };
    let prev_pid: i32 = unsafe { ctx.read_at(24) }.unwrap_or(0);   // prev_pid field
    if let Some(&last) = unsafe { LAST.get(&cpu) } {
        if prev_pid != 0 {                                         // 0 = idle task
            let n = unsafe { BUSY.get(&cpu).copied().unwrap_or(0) } + (now - last);
            let _ = BUSY.insert(&cpu, &n, 0);
        }
    }
    let _ = LAST.insert(&cpu, &now, 0);
    0
}
```

The interval between two switches on a CPU was spent running whatever task
was *leaving* now (`prev`). If that task wasn't the idle task (`prev_pid !=
0`), the CPU was busy for that interval, so we add it to a per-CPU `BUSY`
accumulator. User space samples `BUSY` every couple of seconds, turns the
delta into a **busy percent** per core, and exports
`ebpf_cpu_busy_ns_total{cpu}` — which in Grafana shows a few bars near 100%
and the rest near zero once `scx_nest` is driving.

## Build, deploy, observe

```bash
cd examples/44-scx-nest && ./demo.sh
```

The demo starts `scx_nest`, runs a *moderate* CPU load (fewer busy tasks than
cores — the regime where the nest matters), and attaches the busy probe.
Watch the per-CPU busy percentages: a handful of cores should run hot while
the others stay cool, the nest made visible. Stop `scx_nest` and the same
load spreads back out across all cores under the default scheduler.

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`):

```bash
[vm]$ cat /sys/kernel/sched_ext/root/ops     # active scheduler: nest
[vm]$ mpstat -P ALL 2 1                       # per-CPU utilization: a few hot, rest idle
[vm]$ grep MHz /proc/cpuinfo                  # busy cores clocked higher than idle ones
```

`mpstat` showing a few CPUs busy and the rest idle under a moderate load —
and those busy cores reporting higher clock speeds — is the nest doing its
job; your Aya probe's per-CPU busy series should match it.

## What you learned

- A real scheduling **policy** is an opinion about placement; `scx_nest`
  concentrates work on a small **nest** of warm, high-frequency cores instead
  of spreading it thin.
- The policy is built from the same primitives as `scx_simple` — CPU
  selection and dispatch — plus a **cpumask** (manipulated with
  `bpf_cpumask_*` kfuncs) tracking nest membership, with promotion under load
  and demotion when idle.
- You can **see** a scheduler by measuring per-CPU busy time with an Aya
  `sched_switch` probe: the nest shows up as a few hot cores among idle ones.

That closes the schedulers part's two-chapter tour — minimal and realistic.
Next, Part 8 turns eBPF on real application targets, starting with probing a
production web server.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run (kernel ≥ 6.12): that `scx-scheds` provides
`scx_nest` and it activates, the `sched:sched_switch` `prev_pid` field offset
(24) for the busy accounting, that idle attributes to `prev_pid == 0`, and
that under moderate load the per-CPU busy series (and `mpstat`) show
concentration on a few cores.*
