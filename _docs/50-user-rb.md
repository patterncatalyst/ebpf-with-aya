---
title: "The user ring buffer: a channel into BPF"
order: 50
part: Advanced kernel surface
description: "Every ring buffer so far carried data kernel-to-user. The user ring buffer runs the other way: user space is the producer and the BPF program is the consumer, draining submitted samples with bpf_user_ringbuf_drain and a callback. Learn when to reach for it over a plain map, and meet the dynptr that each sample arrives as."
duration: 40 minutes
---

Ring buffers have appeared all through this book as the kernel's way to stream
events *up* to user space — opensnoop, the L7 probes, the capstone all use one
in that direction. The **user ring buffer** (`BPF_MAP_TYPE_USER_RINGBUF`,
kernel 6.1) is the mirror image: **user space is the producer, and a BPF
program is the consumer.** It's the clean way to feed a *stream* of data into a
running program — and a good lens on a couple of newer primitives, the
`drain`-with-callback pattern and the **dynptr** each sample arrives as.

This is an honest frontier chapter. Aya recognizes the map type, but the
ergonomic pieces — a user-space producer and the kernel-side drain helper —
are still emerging, and the sample callback uses **dynptr**, which this part
covers in its own chapter later. So the canonical C is in
`examples/50-user-rb/reference/`, with an Aya-flavored sketch alongside it; the
`README.md` is candid about what runs today.

{% include excalidraw.html
   file="user-ringbuf"
   alt="User space is the producer: it reserves and submits samples into a user ring buffer (BPF_MAP_TYPE_USER_RINGBUF), the channel running user-to-BPF. A BPF program is the consumer: whenever it runs, it calls bpf_user_ringbuf_drain with a callback, which receives each pending sample and updates an aggregate map (count and sum). The program drains pending samples whenever it runs, and each sample arrives as a dynptr, covered in the dynptr chapter later in this part."
   caption="Figure 50.1 — The ring buffer that runs backwards: user space produces, BPF consumes" %}

## Why not just write a map?

The fair question: user space can already write a regular BPF map that a
program reads, so why a whole new map type? Because that pattern fits a few
*values*, not a *stream*. If user space needs to hand the program an ordered
sequence of variable-length messages — work items, a batch of records,
commands — a shared map turns into a hand-rolled queue with its own locking and
overrun problems. The user ring buffer gives you that queue for free, with the
same reserve/submit discipline as the kernel-to-user ring buffer, and lets the
program consume a whole batch **atomically in its own context** via one drain
call. Rule of thumb: a setting → write a map; a stream → user ring buffer.

## How it works

Two halves, mirror images of the ordinary ring buffer:

- **User space (producer)** reserves space, writes a sample, and submits it —
  `user_ring_buffer__reserve` / `__submit` in libbpf terms. Submitting makes
  the sample visible to the kernel consumer.
- **BPF (consumer)** calls **`bpf_user_ringbuf_drain(&map, callback, ctx,
  flags)`** while it runs. The kernel walks the pending samples and invokes
  your **callback once per sample**; the callback receives the sample as a
  **`bpf_dynptr`** (a bounds-checked handle to a variable-length region), from
  which you pull your record with `bpf_dynptr_data`.

The canonical C makes the consumer side concrete:

```c
struct { __uint(type, BPF_MAP_TYPE_USER_RINGBUF); __uint(max_entries, 256 * 1024); } user_rb SEC(".maps");

static long on_sample(struct bpf_dynptr *dynptr, void *ctx) {
    struct sample *s = bpf_dynptr_data(dynptr, 0, sizeof(*s));
    if (!s) return 0;
    __sync_fetch_and_add(&total_count, 1);
    __sync_fetch_and_add(&total_sum, s->value);
    return 0;                          /* return 1 to stop draining early */
}

SEC("tracepoint/syscalls/sys_enter_getpid")
int drain_it(void *ctx) {
    bpf_user_ringbuf_drain(&user_rb, on_sample, NULL, 0);   /* consume the batch */
    return 0;
}
```

Reading it:

- The program is attached to **some event** — here `getpid`, so the loader can
  trigger draining on demand by calling `getpid()`. In production you'd drain
  on whatever event is relevant; the key idea is that **draining happens when
  the program runs**, not on a timer of its own.
- **`bpf_user_ringbuf_drain`** consumes everything pending in one call,
  invoking `on_sample` per sample. Returning `0` keeps going; returning `1`
  stops early (useful for backpressure).
- Each sample is a **`bpf_dynptr`** — you don't get a raw pointer, you get a
  handle the verifier can bounds-check, and `bpf_dynptr_data` hands you a
  typed, length-checked view. That indirection is what makes variable-length
  user input safe to touch in the kernel.

The Aya rendering follows the same shape — a `UserRingBuf` map, a `drain` call,
a callback — but treat it as a sketch: the producer and drain wrappers are
still settling, and the dynptr accessor is the subject of a later chapter.

## Build, deploy, observe

```bash
cd examples/50-user-rb && ./demo.sh
```

The demo submits a stream of numbers from user space into the ring buffer,
triggers the program to drain them (by calling `getpid` in a loop), and reads
back the aggregate the callback built. **In the terminal** you'll see the count
and sum climb as samples are consumed. **In Grafana** (`127.0.0.1:3000` →
Explore), graph `rate(ebpf_userrb_messages_total[1m])` to watch the consume
rate — user-space-produced data, counted by a kernel program.

## Cross-check

```bash
[vm]$ sudo bpftool map show | grep -i user_ringbuf      # the map type
[vm]$ sudo bpftool map dump name AGG                      # the aggregate the callback built
```

`bpftool` listing a `user_ringbuf` map confirms the channel exists; the `AGG`
map's count matching the number of samples you submitted confirms every
user-space sample reached the kernel consumer.

## What you learned

- The **user ring buffer** reverses the usual flow: **user space produces, a
  BPF program consumes**, draining submitted samples with
  `bpf_user_ringbuf_drain` and a per-sample callback.
- Reach for it over a plain map when you're feeding a **stream** of
  variable-length messages, not a few settings — you get an ordered queue and
  atomic batch consumption for free.
- Each sample arrives as a **`bpf_dynptr`**, a bounds-checked handle — the safe
  way to touch variable-length user input, and a primitive this part returns
  to.

Next, Chapter 51 looks at running eBPF logic in **user space** — turning the
model inside out once more.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): user space produces 1000 samples and the BPF program drains them (count=1000, sum=500500). Uses aya 0.14 + aya-ebpf 0.2.*
