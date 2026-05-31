# 54 · Timers and workqueues: deferred work in the kernel

A BPF program runs to completion and can't sleep, so periodic/deferred work is
done by scheduling a **callback**. This example builds an **in-kernel
per-second aggregator** with a self-rescheduling `bpf_timer` (kernel ≥ 5.15):
the kernel computes the rate, the loader only reads it.

## Pieces

- `reference/timer.bpf.c` — canonical C: `count` (bumps a counter), `arm`
  (init/set_callback/start once), `tick` (softirq callback: snapshot + re-arm).
- `timer-ebpf` — Aya rendering (sketch: the `bpf_timer` lifecycle maps to
  helpers, but callback-as-subprogram ergonomics are rough).
- `timer-common` — the `Slot { count, rate, struct bpf_timer }`.
- `timer` — holds the map open (required), arms once, drives getpid, reads
  `rate`; exports `ebpf_timer_events_per_sec`.

## Run it

```bash
./demo.sh          # kernel computes the per-second rate; loader reads it
./demo.sh build
```

## Timer vs workqueue

- **timer** (`bpf_timer_*`): softirq, **non-sleepable**, periodic — rate
  windows, TTL expiry.
- **workqueue** (`bpf_wq_*`, kernel ≥ 6.10): process context, **sleepable** —
  fast-path/slow-path, deferring expensive/blocking work.

## Cross-check

```bash
sudo bpftool map dump name slots         # rate snapshots each second while you only read
sudo bpftool prog show | grep -i timer   # the program stays loaded with a pending timer
```

## Verification status

**Unverified.** Confirm the `bpf_timer_*` helper lifecycle and re-arm (≥ 5.15);
that holding the map open satisfies the user-reference requirement; and treat
the aya-ebpf callback rendering as a sketch — the C reference is canonical.
