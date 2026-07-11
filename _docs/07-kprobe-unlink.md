---
title: "kprobe + unlink"
order: 7
part: Tracing the kernel
description: Your first kprobe — attach to the kernel function behind file deletion, read the calling process's identity, attempt to read the filename argument, and stream per-event records to user space via a ring buffer.
duration: 30 minutes
---

Chapter 6's tracepoint fired on a *stable* kernel trace event. A
**kprobe** is more powerful and more dangerous: it attaches to (almost)
any kernel function by name, at its entry, giving you the function's
arguments. This chapter builds `unlinksnoop` — a kprobe on
`vfs_unlink()`, the VFS-layer function behind `unlink()`/`unlinkat()` — that
reports who deleted what. Along the way you meet ring buffers (the
modern way to stream events to user space) and the reality that reading
kernel struct fields is where eBPF gets version-sensitive.

The code is in `examples/07-kprobe-unlink/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

## kprobe vs tracepoint

A **tracepoint** is a stable instrumentation point the kernel
developers placed and promised not to break — it has a documented
argument format. A **kprobe** attaches to an arbitrary kernel function;
it's enormously flexible (any function, no waiting for someone to add a
tracepoint) but the function's name, signature, and the layout of the
structs it's passed are **implementation details that change between
kernel versions**. That trade-off — flexibility for stability — is the
whole story of this chapter, and the reason Chapter 8 revisits the same
target with `fentry`, which gets the best of both.

We probe `vfs_unlink(struct mnt_idmap *, struct inode *dir, struct
dentry *dentry, …)`. It's a good first kprobe: it fires whenever anything
deletes a file, the calling process context is easy to read, and its
`dentry` argument is a pointer we can *try* to follow to the filename — a
concrete lesson in reading kernel memory.

> This chapter first targeted `do_unlinkat()`, but newer kernels **inlined
> that helper away** — it's gone from `kallsyms` and BTF, so the kprobe can
> no longer attach to it. That's a live demonstration of the very fragility
> described above, and exactly why we moved to `vfs_unlink`, the stable
> VFS-layer function the standard tools (bcc, bpftrace) hook. The trade-off
> for that stability: `vfs_unlink` hands you a `dentry`, not a `struct
> filename *`, so the name now comes from `dentry->d_name.name` (a
> version-specific struct offset — see the code).

## The kernel side

The handler (`unlinksnoop-ebpf/src/main.rs`) does three things: reserve
a ring-buffer slot, fill in process context, attempt to read the
filename, submit.

```rust
#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[kprobe]
pub fn vfs_unlink(ctx: ProbeContext) -> u32 {
    // reserve -> fill -> submit
}
```

### Stable process context (always works)

These helpers are available in every program context and never depend
on kernel struct layout:

```rust
let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
let uid = (bpf_get_current_uid_gid() & 0xffff_ffff) as u32;
(*event).comm = bpf_get_current_comm().unwrap_or([0u8; 16]);
```

`pid_tgid` packs the thread ID in the low bits and the process ID
(`tgid`) in the high bits — we shift to get the PID a human cares about.
`comm` is the 16-byte process name. If you only ever read these, your
kprobe is as portable as a tracepoint.

### Reading the argument (the version-sensitive part)

The interesting, fragile bit is the filename. `vfs_unlink`'s third
argument (index 2) is a `struct dentry *`, and the unlinked name lives at
`dentry->d_name.name`. Aya hands you arguments via `ctx.arg::<T>(n)`:

```rust
// dentry->d_name.name — on this kernel d_name is at offset 32 (a struct
// qstr) and qstr.name is at offset 8, so the name pointer sits at + 40.
if let Some(dentry) = ctx.arg::<*const u8>(2) {
    let name_pptr = dentry.add(32 + 8) as *const *const u8;
    if let Ok(p) = bpf_probe_read_kernel::<*const u8>(name_pptr) {
        let _ = bpf_probe_read_kernel_str_bytes(p, &mut (*event).filename);
    }
}
```

Two things deserve emphasis. First, you **cannot** dereference a kernel
pointer directly — the verifier forbids it. You must copy through
`bpf_probe_read_kernel`, which safely reads kernel memory into your
stack/map. Second, we read the name pointer at a **fixed offset** into
`struct dentry` (`d_name.name`); those offsets hold on this kernel but are
exactly the kind of assumption a future kernel can invalidate — indeed the
helper this chapter first targeted, `do_unlinkat`, was removed outright,
which is why we now hook `vfs_unlink`. If the read fails, we leave
the filename empty and still emit the event — **degrade, don't crash**.
The robust answer to this fragility is CO-RE (BTF-relocated field
access), which `fentry` (Chapter 8) and the CO-RE deep-dive
(Part 9) build out properly. For a first kprobe, the explicit
`probe_read` makes the underlying mechanic visible.

### Ring buffers

Chapter 6 used a counter (a single number). Here each unlink is a
distinct *event* with fields, so we stream records through a
**`RingBuf`** — a shared, kernel-to-user circular buffer that's the
modern replacement for `PerfEventArray`. The kernel `reserve`s a slot,
writes into it, and `submit`s; user space drains completed records.
It's lossy under extreme pressure (the kernel drops rather than blocks)
and lock-free, which is what you want on a hot path.

## The user side

`unlinksnoop/src/main.rs` attaches the kprobe by function name and
drains the ring:

```rust
let program: &mut KProbe = ebpf.program_mut("vfs_unlink").unwrap().try_into()?;
program.load()?;
program.attach("vfs_unlink", 0)?;     // 0 = entry offset
```

Note the symmetry with Chapter 6 — same load/attach arc, just a
`KProbe` and a function name instead of a `TracePoint` and an event
name. Draining uses a simple tokio timer that pulls all available
records each tick:

```rust
let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;
loop {
    tokio::time::sleep(Duration::from_millis(200)).await;
    while let Some(item) = ring.next() {
        let ev: UnlinkEvent = unsafe { core::ptr::read_unaligned(item.as_ptr() as *const _) };
        println!("{} {} {} {}", ev.pid, ev.uid, cstr(&ev.comm), cstr(&ev.filename));
        counter.add(1, &[KeyValue::new("program", "unlinksnoop")]);
    }
}
```

Poll-on-timer is simple and robust; the more efficient approach
registers the ring's file descriptor with `tokio`'s `AsyncFd` and wakes
only when data is ready. Start simple; optimize if a chapter's event
rate demands it.

The shared `UnlinkEvent` is `#[repr(C)]` in `unlinksnoop-common` so the
bytes the kernel writes line up exactly with what user space reads.
This is the contract from Chapter 5 made concrete — a wrong field order
here reads as garbage, not an error.

## Build, deploy, observe

```bash
cd examples/07-kprobe-unlink && ./demo.sh
```

It builds on the host, ships the binary to `ebpf-target`, and runs it.
Generate deletions on the target in another terminal:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'for i in $(seq 1 20); do t=$(mktemp); rm -f "$t"; done'
```

A `PID UID COMM FILE` table fills in, and `ebpf_events_total` climbs on
the Grafana overview dashboard with the `unlinksnoop` label.

**In Grafana** (`127.0.0.1:3000` → Explore), filter to the `ebpf-unlinksnoop` service and graph `sum by (program) (rate(ebpf_events_total[1m]))` — unlink/delete attempts as they happen as a live rate, the same events your terminal lists, now plotted over time.

## Cross-check against the kernel

On the target VM, confirm independently:

```bash
[vm]$ sudo bpftrace -e 'kprobe:vfs_unlink { @[comm] = count(); }'
```

`bpftrace` attaches its own kprobe to the same function and counts per
process. If its counts track the table `unlinksnoop` prints, both your
attach point and your event plumbing are correct. If `bpftrace` sees
unlinks but `unlinksnoop` shows empty filenames, you've isolated the
bug to the argument-reading section — exactly the version-sensitive
part flagged above.

## What you learned

- A kprobe attaches to a kernel function by name; the cost is
  sensitivity to kernel internals.
- Process context (`pid`/`uid`/`comm`) is portable; reading struct
  fields off arguments is not — copy via `bpf_probe_read_kernel`, and
  degrade gracefully.
- Ring buffers stream per-event records; user space drains them.

Next, the same `unlink` target with **`fentry`** — lower overhead than
a kprobe, with typed, BTF-relocated argument access that fixes the
fragility you just met.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
