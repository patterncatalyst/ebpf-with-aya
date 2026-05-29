# Example 25 — biopattern (sequential vs. random block I/O)

Classify block-device I/O as **sequential** or **random**, per device —
the access pattern that decides whether a disk is happy or thrashing.

## What this shows

- A **block-layer tracepoint** (`block:block_rq_issue`) and reading its
  `dev` / `sector` / `nr_sector` fields by offset.
- **Per-device state in kernel**: track each device's last end-sector; a
  request whose start sector equals the previous end is *sequential*,
  otherwise *random*. Accumulate per-device counters (no per-I/O events).
- Exporting the **sequential ratio** per device to Grafana via an OTLP
  observable gauge, and printing a live table.

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # deploy + run (per-device table every 2s; Ctrl-C to stop)
```

Drive contrasting workloads on the VM:

```bash
# sequential
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'dd if=/dev/zero of=/tmp/seq bs=1M count=512 oflag=direct; sync'
# random (needs fio: sudo dnf install -y fio)
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'fio --name=rand --filename=/tmp/seq --rw=randread --bs=4k --size=256m --direct=1 --runtime=20 --time_based'
```

The `dd` run should push SEQ% high; the `fio` random read should drag it
down. Map `DEV` (major:minor) to disks with `lsblk` / `/proc/partitions`.

## Cross-check (on the VM)

```bash
[vm]$ sudo biopattern-bpfcc
[vm]$ sudo biosnoop-bpfcc        # per-I/O detail, incl. sector + LBA
```

## ⚠ Verification status

**Unverified.** Risks: the `block_rq_issue` field offsets (`dev`@8,
`sector`@16, `nr_sector`@24 — verify vs. the format file; layout has
changed across kernels); the `dev_t` major/minor decoding; `HashMap`
read-modify-write for per-device counters; and the OTLP observable-gauge
API in opentelemetry 0.27. `block_rq_complete` is an alternative
attach point (counts finished I/O). Record results in
`_plans/reconciliation-plan.md`.
