---
title: "biopattern"
order: 25
part: Performance & resources
description: "Classify block-device I/O as sequential or random per device by tracking each device's sector position across the block_rq_issue tracepoint — the access pattern that determines whether storage thrives or thrashes."
duration: 25 minutes
---

Storage performance hinges on one thing more than almost any other:
whether I/O is **sequential** or **random**. Spinning disks fall off a
cliff on random access; even SSDs prefer sequential. `biopattern`
measures the split per device, so you can tell a streaming workload from
a seeky one at a glance. It's a compact lesson in block-layer tracing and
per-device kernel state.

The code is in `examples/25-biopattern/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="bio-seq-random"
   alt="Sequential vs random I/O: on each block_rq_issue, if the request's starting sector equals the previous request's end sector it is sequential, otherwise random, tallied per device in a kernel HashMap."
   caption="Figure 25.1 — classifying block I/O as sequential or random" %}

## Sequential vs. random, defined by sectors

A block request targets a starting **sector** and spans `nr_sector`
sectors. The pattern is just geometry: if a request begins exactly where
the previous one on the same device **ended**, the head/controller
didn't have to seek — that's **sequential**. If it starts somewhere
else, that's **random**. So all we need is, per device, the end sector
of the last request:

```text
this request: [sector, sector + nr_sector)
sequential  ⇔  sector == last_end_sector[dev]
last_end_sector[dev] = sector + nr_sector
```

## The block tracepoint

`block:block_rq_issue` fires when a request is submitted to a device. We
read three fields by offset (from its format file):

```rust
let dev       = ctx.read_at::<u32>(OFF_DEV)?;        // @ 8
let sector    = ctx.read_at::<u64>(OFF_SECTOR)?;     // @ 16
let nr_sector = ctx.read_at::<u32>(OFF_NR_SECTOR)?;  // @ 24
```

Block tracepoint layouts have shifted across kernel versions, so these
offsets are the first thing to verify against
`/sys/kernel/debug/tracing/events/block/block_rq_issue/format` on your
Fedora 44 kernel.

## Per-device state in the kernel

Two maps keyed by device id (`dev_t`): `LAST_END[dev]` holds the previous
request's end sector, and `STATS[dev]` accumulates `{ sequential, random,
bytes }`. Each issue updates both — entirely in-kernel, no per-I/O events
(block I/O is high-frequency, so this is the same aggregate-in-kernel
discipline as `runqlat` and `hardirqs`):

```rust
let is_seq = LAST_END.get(&dev).map_or(false, |&end| sector == end);
LAST_END.insert(&dev, &(sector + nr_sector), 0);
// bump STATS[dev].sequential / .random, add nr_sector*512 to .bytes
```

Keying by device matters: interleaved I/O to *different* disks shouldn't
look random just because the requests alternate. Per-device `LAST_END`
keeps each disk's classification accurate.

## Naming devices

The `dev` field is a `dev_t` — a packed major/minor number. We decode it
to `major:minor` for display; map that to a disk name with `lsblk` or
`/proc/partitions` on the VM (e.g. `253:0` → `vda`). User space prints a
live per-device table and exports `bio_sequential_ratio{dev}` as an OTLP
observable gauge — a Grafana line per disk that drops when a workload
turns seeky.

## Build, deploy, observe

```bash
cd examples/25-biopattern && ./demo.sh
```

Then run contrasting workloads on the VM:

```bash
# sequential: a big streaming write
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'dd if=/dev/zero of=/tmp/seq bs=1M count=512 oflag=direct; sync'
# random: 4 KiB reads all over the file (needs fio)
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'fio --name=rand --filename=/tmp/seq --rw=randread --bs=4k --size=256m --direct=1 --runtime=20 --time_based'
```

Watch SEQ% sit high during the `dd`, then collapse during the random
`fio` run — the pattern made visible.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `ebpf_bio_sequential_ratio` (a gauge — no `rate()`) — the sequential-vs-random ratio of block I/O (0–1).

## Cross-check

```bash
[vm]$ sudo biopattern-bpfcc
[vm]$ sudo biosnoop-bpfcc       # per-I/O: sector, size, latency
```

## What you learned

- **Sequential vs. random** is defined by whether a request starts where
  the last one ended — pure sector arithmetic.
- Trace it at **`block:block_rq_issue`**, reading `dev`/`sector`/`nr_sector`.
- Keep **per-device state** (`LAST_END`, `STATS`) in kernel and
  aggregate — don't emit per-I/O.
- Decode `dev_t` to `major:minor` and map to disks with `lsblk`.

That's the heart of *Performance & resources*' tooling. Next, the part
closes with a forward-looking chapter on **energy/power monitoring**
with eBPF.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
