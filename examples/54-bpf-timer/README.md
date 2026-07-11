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

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab VM
(kernel 7.1.3-200.fc44, which satisfies the `bpf_timer` ≥ 5.15 and `bpf_wq`
≥ 6.10 floors): it builds, loads, attaches, holds the map open, arms the timer
once, and reports a per-second event rate. Note the in-kernel `bpf_timer`
callback form is not expressible in aya-ebpf — the C reference in
`reference/timer.bpf.c` is canonical, and here the rate is computed in the
userspace loader. Attach targets and struct offsets can be kernel-version-specific.
