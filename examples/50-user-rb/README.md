# 50 · The user ring buffer: a channel into BPF

`BPF_MAP_TYPE_USER_RINGBUF` (kernel 6.1) reverses the usual flow: **user space
produces, a BPF program consumes**. The consumer calls `bpf_user_ringbuf_drain`
and a callback runs once per submitted sample (arriving as a `bpf_dynptr`).

> **Experimental.** Aya knows the map type, but the user-space producer and the
> kernel-side drain + dynptr accessor are still settling. `reference/
> user_ringbuf.bpf.c` is the canonical form; `user-rb-ebpf` is an Aya sketch.

## Pieces

- `reference/user_ringbuf.bpf.c` — canonical C consumer (read-only).
- `user-rb-ebpf` — Aya sketch: `UserRingBuf` map + `drain(callback)` on a
  `getpid` tracepoint, aggregating into `AGG`.
- `user-rb-common` — the shared `Sample`.
- `user-rb` — producer: submits 1000 samples, triggers draining via `getpid`,
  reads back `AGG`; exports `ebpf_userrb_messages_total`.

## Run it

```bash
./demo.sh          # produce a stream, drain it in-kernel, read the aggregate
./demo.sh build    # just build on the host
```

Expected: `count=1000 sum=500500` if every submitted sample reached the kernel.

## Cross-check

```bash
sudo bpftool map show | grep -i user_ringbuf
sudo bpftool map dump name AGG
```

## Verification status

**Unverified / experimental** (kernel ≥ 6.1). Confirm Aya's user-ringbuf
producer + drain/dynptr support (C reference is canonical), that draining on
`sys_enter_getpid` consumes samples, and that `AGG` count matches the number
submitted.
