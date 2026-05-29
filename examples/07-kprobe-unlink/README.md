# Example 07 — kprobe + unlink (`unlinksnoop`)

Your first **kprobe**: attach to the kernel function `do_unlinkat()`,
which sits behind `unlink()`/`unlinkat()`, and report who deletes what.

## What this shows

- A `#[kprobe]` program attached to a kernel function by name
  (`do_unlinkat`) — the defining difference from Chapter 6's tracepoint.
- Reading **stable process context** in-kernel: `pid`, `uid`, `comm`
  via always-available helpers.
- Reading a **kprobe function argument** (`struct filename *`) and
  following it to the path string — the version-sensitive part, done
  with `bpf_probe_read_kernel`, failing gracefully if the layout
  differs.
- Streaming per-event records to user space via a **`RingBuf`** (the
  modern successor to `PerfEventArray`), drained on a tokio timer.
- Exporting `ebpf_events_total{program="unlinksnoop"}` to the stack.

## Layout

```text
unlinksnoop-common/   # #[repr(C)] UnlinkEvent shared by both halves
unlinksnoop-ebpf/     # the kprobe handler -> RingBuf
unlinksnoop/          # user space: attach, drain ring, print + export OTLP
  build.rs            # compiles the ebpf crate (aya-build)
```

## Run it

Prereqs: toolchain (Ch 4), `ebpf-target` VM up (Ch 2), stack up (Ch 3).

```bash
./demo.sh build      # just build on the host
./demo.sh            # build + deploy to the VM + run (Ctrl-C to stop)
```

Generate unlink traffic on the target (in another terminal):

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'for i in $(seq 1 20); do t=$(mktemp); rm -f "$t"; done'
```

You'll see a `PID UID COMM FILE` table fill in, and
`ebpf_events_total` climb on the Grafana overview dashboard.

## Cross-check (on the VM)

```bash
[vm]$ sudo bpftool prog list | grep -A3 kprobe
[vm]$ sudo bpftrace -e 'kprobe:do_unlinkat { @[comm] = count(); }'
```

The `bpftrace` counts per-process unlinks independently — compare
against the table `unlinksnoop` prints.

## ⚠ Verification status

**Unverified.** Written to current Aya conventions (`aya` 0.13.x,
`aya-ebpf` 0.1.x) but not compiled/run at authoring. The two parts most
likely to need adjustment on real hardware:

1. **The filename read.** `do_unlinkat`'s 2nd arg is `struct filename *`;
   we assume the path pointer is its first field. If your kernel's
   layout differs, the read fails gracefully (empty filename) — the
   pid/uid/comm still report. The robust fix is CO-RE field access via
   BTF, introduced properly in the `fentry` chapter (8) and the CO-RE
   deep-dive (56).
2. **`RingBuf` draining.** The poll-on-timer approach here is simple and
   robust; the more efficient `AsyncFd`-based approach is an
   optimization noted in the chapter.

Record results in `_plans/reconciliation-plan.md`.
