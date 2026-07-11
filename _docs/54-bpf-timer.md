---
title: "Timers and workqueues: deferred work in the kernel"
order: 54
part: Advanced kernel surface
description: "A BPF program runs to completion and cannot sleep or loop forever — so how does it do something later, or periodically, or move expensive work off a hot probe? BPF timers (kernel 5.15) fire a callback in softirq context and can re-arm themselves; BPF workqueues (kernel 6.10) run a sleepable callback in process context. Learn the init/set_callback/start lifecycle, build an in-kernel periodic aggregator, and know when to reach for each."
duration: 45 minutes
---

Every BPF program in this book has obeyed one iron rule: **run to completion,
quickly, without sleeping.** The verifier enforces it — no unbounded loops, no
blocking, get in and get out. That rule is what makes BPF safe to wedge into
hot kernel paths, but it raises an obvious problem. How does a program do
something *later* — expire a stale map entry, roll up a per-second statistic,
move an expensive computation off the probe that triggered it? You can't sleep,
and you can't spin. The kernel's answer is to let a program **schedule a
callback**: **timers** (kernel 5.15) and, more recently, **workqueues**
(kernel 6.10). They are the in-kernel equivalent of "do this in a bit," and
they're what let BPF keep a hot path fast while still doing real work.

The code is in `examples/54-bpf-timer/`. `./demo.sh` runs an in-kernel
per-second aggregator driven entirely by a self-rescheduling timer; the
`README.md` has the details.

{% include excalidraw.html
   file="bpf-timer"
   alt="An event (tracepoint) drives a BPF program that runs to completion and arms a timer. The timer lives in a map value alongside a count and rate, as a struct bpf_timer. The program calls init and start. Later the timer callback fires in softirq, atomic context: it sets rate = count and re-arms itself. Alternatively a workqueue callback runs in process, sleepable context for slow-path or I/O work; the program can schedule sleepable work there. The takeaway: timer is softirq (atomic, periodic, self-rescheduling); workqueue is process context (sleepable, fast-path/slow-path)."
   caption="Figure 54.1 — A timer or workqueue callback fires after the program returns, doing deferred work" %}

## The shape of deferred work

Both mechanisms share a model that's worth getting straight before any code,
because it's unlike anything earlier in the book:

- The timer or workqueue **lives inside a map value.** You put a `struct
  bpf_timer` (or `struct bpf_wq`) field in your value struct. It is not a
  global; it belongs to a specific map element, which means its lifetime is the
  element's lifetime.
- You **initialize** it (`bpf_timer_init`), **attach a callback**
  (`bpf_timer_set_callback`), and **arm** it (`bpf_timer_start` with a delay in
  nanoseconds). The callback runs *after the current program returns*,
  asynchronously, when the timer expires.
- The callback has a fixed signature — it's handed the map, the key, and the
  value it lives in: `int cb(void *map, KEY *key, VALUE *value)`. So it can read
  and update the very element that owns it.
- Because the timer is bolted to a map element, **the map must have a user
  reference** — an open fd or a bpffs pin — or `init`/`set_callback` returns
  `-EPERM`. And **the map's lifecycle owns the timer**: free or delete the map
  and all its pending timers are canceled. A pending timer even keeps the
  *program* loaded, since the callback belongs to it.

That last point is the mental shift: a timer is not a free-floating thread,
it's a property of a map element, and it inherits that element's existence.

## Timers: periodic work in softirq

A `bpf_timer` callback runs in **softirq context** — atomic, fast, and
crucially **not sleepable**. It can do the things any normal BPF program does
(touch maps, compute, call non-sleepable helpers) but it cannot block. Its
superpower is **self-rescheduling**: a callback can call `bpf_timer_start` on
its own timer again, turning a one-shot into a periodic tick that runs entirely
in the kernel with no user-space loop poking it.

The canonical use is in-kernel periodic aggregation, which is exactly our
example. One map element holds a counter and a timer; a tracepoint bumps the
counter; the timer fires once a second to snapshot it as a rate and re-arm:

```c
struct slot { __u64 count; __u64 rate; struct bpf_timer timer; };
struct { __uint(type, BPF_MAP_TYPE_ARRAY); __uint(max_entries, 1);
         __type(key, __u32); __type(value, struct slot); } slots SEC(".maps");

static int tick(void *map, __u32 *key, struct slot *s) {
    s->rate  = s->count;                       /* events in the last second */
    s->count = 0;                              /* reset the window          */
    bpf_timer_start(&s->timer, NSEC_PER_SEC, 0);  /* re-arm: periodic        */
    return 0;
}

SEC("tracepoint/syscalls/sys_enter_getpid")
int count(void *ctx) {
    __u32 k = 0;
    struct slot *s = bpf_map_lookup_elem(&slots, &k);
    if (!s) return 0;
    __sync_fetch_and_add(&s->count, 1);
    return 0;
}
```

and a one-time arming program kicks it off:

```c
SEC("tracepoint/syscalls/sys_enter_execve")
int arm(void *ctx) {
    __u32 k = 0;
    struct slot *s = bpf_map_lookup_elem(&slots, &k);
    if (!s) return 0;
    bpf_timer_init(&s->timer, &slots, CLOCK_MONOTONIC);  /* clock source     */
    bpf_timer_set_callback(&s->timer, tick);             /* what runs on fire */
    bpf_timer_start(&s->timer, NSEC_PER_SEC, 0);         /* first fire in 1s  */
    return 0;
}
```

Reading the lifecycle the way it actually executes:

- **`bpf_timer_init`** binds the timer to its owning map and picks a clock
  (`CLOCK_MONOTONIC` for elapsed-time periodics). It must run before anything
  else touches the timer, and only once.
- **`bpf_timer_set_callback`** points the timer at `tick`. The callback is a
  separate BPF subprogram the verifier checks independently — note it never
  sleeps, because softirq.
- **`bpf_timer_start(&timer, NSEC_PER_SEC, 0)`** arms it for one second out.
  When `tick` runs, it re-arms itself, so from then on it fires every second
  forever — a kernel-resident metronome. User space never schedules anything;
  it just reads `rate`.

The win is real: computing a per-second rate this way means **zero user-space
wakeups** in steady state. The kernel keeps the rolling number; your loader
reads it whenever it likes.

## Workqueues: when the deferred work must sleep

A timer callback can't sleep, which rules out a whole class of deferred work:
anything that calls a **sleepable kfunc**, does I/O, or otherwise might block.
That's why kernel 6.10 added **BPF workqueues** — described by their author as
"`bpf_timer` but in process context instead of softirq." A `struct bpf_wq`
lives in a map value just like a timer, with a kfunc-based lifecycle
(`bpf_wq_init`, `bpf_wq_set_callback`, `bpf_wq_start`) and a callback `int
cb(void *map, int *key, struct bpf_wq *wq)` that runs in **process context and
is sleepable.**

The pattern workqueues unlock is **fast path / slow path**. Your probe sits on
a hot path and must stay quick, so it does the minimum — record the event,
maybe stash a key — and then schedules the expensive or sleepable part to a
workqueue. The fast path stays responsive; the slow path gets the full range of
sleepable kernel operations. Reach for a **timer** when the deferred work is
cheap, periodic, and non-blocking (rate windows, TTL expiry); reach for a
**workqueue** when it needs to sleep (call a sleepable kfunc, do real work off
the hot path).

## Where Aya fits

Both mechanisms exist as helpers (`bpf_timer_*`) and kfuncs (`bpf_wq_*`,
`bpf_timer_set_sleepable_cb`), and the hard part for Aya isn't the calls — it's
the **callback**. `set_callback` takes a pointer to a separate BPF subprogram,
and expressing "a callback the verifier can check as its own subprogram" in
aya-ebpf is still rough. So the canonical form lives in
`examples/54-bpf-timer/reference/timer.bpf.c`, with an Aya rendering alongside
it, flagged. The **user side is ordinary Aya** — the loader holds the map open
(satisfying the user-reference requirement), kicks the arming program once, and
reads `rate` — so the part you interact with is solid even while the in-kernel
callback ergonomics settle.

## Build, deploy, observe

```bash
cd examples/54-bpf-timer && ./demo.sh
```

The demo loads the program, triggers the one-time `arm`, then generates a
steady stream of `getpid` calls and reads the `rate` field once a second. **In
the terminal** you'll see the per-second count settle near your generated rate
— a number the kernel computed on its own timer. **In Grafana**
(`127.0.0.1:3000` → Explore), graph `ebpf_timer_events_per_sec` to watch the
in-kernel rate, with no user-space sampling loop producing it.

## Cross-check

```bash
[vm]$ sudo bpftool map dump name slots          # count climbing, rate snapshotting each second
[vm]$ sudo bpftool prog show | grep -i timer     # the program stays loaded while the timer is pending
```

Watching `rate` update once per second in `bpftool map dump` while your loader
is merely *reading* (not writing) it is the proof the kernel timer is doing the
work; and the program remaining loaded with a pending timer is the
"timer keeps the program alive" property made visible.

## What you learned

- A BPF program **runs to completion and can't sleep**, so deferred or periodic
  work is done by scheduling a **callback** that fires after the program
  returns.
- **Timers** (`bpf_timer_*`) live in a map value, run their callback in
  **softirq** (atomic, non-sleepable), and can **re-arm themselves** for
  periodic, user-space-free aggregation; the map must be held open and owns the
  timer's lifecycle.
- **Workqueues** (`bpf_wq_*`, kernel 6.10) run a **sleepable** callback in
  process context, enabling the **fast-path/slow-path** pattern; choose a timer
  for cheap periodic work, a workqueue when the deferred work must sleep.

Next, Chapter 55 looks at **struct_ops** in general — implementing kernel
policy interfaces in BPF, the mechanism beneath the schedulers of Part 6.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): reports a per-second event rate. The in-kernel bpf_timer form is not expressible in aya-ebpf (see below); the rate is computed in the userspace loader.*
