---
title: "opensnoop"
order: 9
part: Tracing the kernel
description: Trace every file open with stable syscall tracepoints — read tracepoint arguments by offset, learn the user-vs-kernel memory-read distinction, and pair entry and exit to report which process opened which file with what result.
duration: 30 minutes
---

Chapters 7 and 8 attached to a kernel *function*. This chapter attaches
to **tracepoints** — stable, documented instrumentation points the
kernel developers maintain — and builds `opensnoop`, the classic tool
that shows every file open on the system: which process, which file,
what flags, success or failure. Along the way you learn to read
tracepoint arguments by offset and meet a distinction that bites
people: **user memory versus kernel memory**.

The code is in `examples/09-opensnoop/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="mem-read-user-kernel"
   alt="User vs kernel memory: an eBPF program may not dereference raw pointers; it copies bytes from the process's user memory with bpf_probe_read_user and from kernel memory with bpf_probe_read_kernel."
   caption="Figure 9.1 — reading user vs kernel memory (the basis for chapters 9–12)" %}

## Why tracepoints here

Syscalls have stable tracepoints (`syscalls:sys_enter_openat`,
`syscalls:sys_exit_openat`) with a *documented argument format* that the
kernel commits to keeping stable. That's the opposite trade-off from a
kprobe: less reach (only where tracepoints exist), but no fragility
from kernel-internal struct layouts. For syscall tracing — which is
most observability work — tracepoints are the right default.

`opensnoop` watches `openat`, the syscall behind essentially all file
opens on a modern system. We pair the **enter** tracepoint (which has
the filename and flags) with the **exit** tracepoint (which has the
return value: the new fd, or a negative errno) using the same
`pid_tgid` `HashMap` bridge from Chapter 8.

## Reading tracepoint arguments by offset

A tracepoint hands your program a record whose fields sit at fixed
byte offsets. The kernel publishes the layout. On the target VM:

```bash
[vm]$ cat /sys/kernel/tracing/events/syscalls/sys_enter_openat/format
```

You'll see fields with `offset:` annotations — after the 8-byte common
header and the `__syscall_nr` field, the openat args begin: `dfd`@16,
`filename`@24, `flags`@32, `mode`@40. The exit record has `ret`@16.
Aya reads them from the context:

```rust
let filename_ptr: *const u8 = ctx.read_at(24)?;   // const char *filename
let flags: i32 = ctx.read_at::<i32>(32)?;          // int flags
```

These offsets are long-stable on x86_64, but they are exactly the kind
of value to *verify against the format file* rather than trust blindly
— which is why the chapter has you read it. That's the
tracepoint discipline: stable, but confirm.

## User memory vs. kernel memory

Here's the subtlety. In Chapters 7–8 we read kernel pointers with
`bpf_probe_read_kernel`. But at *syscall entry*, the `filename` argument
is a pointer into the **calling process's user-space memory** — the
program hasn't copied it into the kernel yet. Reading it needs the
*user* variant:

```rust
bpf_probe_read_user_str_bytes(filename_ptr, &mut ev.filename);
```

Use the kernel reader on a user pointer (or vice versa) and you get an
error or garbage. The rule of thumb: **syscall arguments at entry are
user memory; kernel structures are kernel memory.** Getting this right
is half of writing correct syscall probes.

## How the code works

### Two maps, two jobs

The kernel side (`opensnoop-ebpf/src/main.rs`) declares two maps, and the
choice of type for each is the whole design:

```rust
#[map] static INFLIGHT: HashMap<u64, OpenInfo> = HashMap::with_max_entries(10240, 0);
#[map] static EVENTS:   RingBuf               = RingBuf::with_byte_size(256 * 1024, 0);
```

`INFLIGHT` is a `HashMap` keyed by `pid_tgid` — scratch space that holds
what we learned at *entry* until the matching *exit* fires. `EVENTS` is a
`RingBuf` — the one-way kernel→user channel for *completed* opens. We
don't emit at entry because we don't yet know if the open succeeded; we
don't keep state in the ring buffer because a ring buffer isn't
addressable by key. Each map type is doing the one thing it's good at.

### Entry: stash what only entry knows

```rust
#[tracepoint]
pub fn sys_enter_openat(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let filename_ptr: *const u8 = match unsafe { ctx.read_at(24) } { Ok(p) => p, _ => return 0 };
    let flags: i32 = unsafe { ctx.read_at(32) }.unwrap_or(0);

    let mut info = OpenInfo { flags, filename: [0u8; NAME_LEN] };
    unsafe { let _ = bpf_probe_read_user_str_bytes(filename_ptr, &mut info.filename); }
    let _ = INFLIGHT.insert(&pid_tgid, &info, 0);
    0
}
```

Walking it: `bpf_get_current_pid_tgid()` returns a 64-bit value packing
the thread id (low 32) and process id (high 32) — it's our correlation
key *and* it's unique per in-flight syscall, because a thread can only be
inside one `openat` at a time. `ctx.read_at(24)` pulls the `filename`
argument out of the tracepoint record at the offset we read from the
format file; note we get a *pointer*, not the string — the string lives
in user memory (Figure 9.1), so `bpf_probe_read_user_str_bytes` copies it
into our stack-allocated `info.filename`. That helper is the user-memory
analogue of the kernel reader from Chapter 8, and it stops at the NUL or
the buffer end, so it's bounded — which is what keeps the verifier happy.
Finally `INFLIGHT.insert` stashes the struct under `pid_tgid`. Nothing is
reported yet.

### Exit: pair, decide, emit

```rust
#[tracepoint]
pub fn sys_exit_openat(ctx: TracePointContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let ret: i64 = unsafe { ctx.read_at(16) }.unwrap_or(-1);
    let info = match unsafe { INFLIGHT.get(&pid_tgid) } { Some(i) => *i, None => return 0 };
    let _ = INFLIGHT.remove(&pid_tgid);

    if let Some(mut slot) = EVENTS.reserve::<OpenEvent>(0) {
        let ev = OpenEvent {
            pid: (pid_tgid >> 32) as u32,
            uid: (bpf_get_current_uid_gid() & 0xffff_ffff) as u32,
            ret, comm: bpf_get_current_comm().unwrap_or_default(),
            flags: info.flags, filename: info.filename,
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    0
}
```

The exit record carries the **return value** at offset 16 — the one
thing entry couldn't know. `INFLIGHT.get(&pid_tgid)` finds the entry we
stashed; if it's missing (we started tracing mid-syscall) we bail. We
`remove` immediately so the map doesn't leak entries for syscalls we've
already reported. Then the emit pattern you'll see in every event-based
chapter: `EVENTS.reserve::<OpenEvent>(0)` claims a slot *in* the ring
buffer sized for our struct (no copy, no allocation — you write straight
into kernel ring memory), we fill it — completing the record with `pid`,
`uid`, and `comm` that are cheap to get at exit — and `slot.submit(0)`
publishes it so user space can see it. If `reserve` returns `None` the
ring is full and we simply drop the event rather than block; losing an
event is always preferable to stalling the kernel.

### User space: attach both, drain one

```rust
for (name, cat, tp) in [("sys_enter_openat", "syscalls", "sys_enter_openat"),
                        ("sys_exit_openat",  "syscalls", "sys_exit_openat")] {
    let p: &mut TracePoint = ebpf.program_mut(name).unwrap().try_into()?;
    p.load()?;
    p.attach(cat, tp)?;
}
let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;
```

`program_mut(name)` looks the program up by the kernel function's name;
the `try_into::<&mut TracePoint>()` is where Aya checks the program is
actually a tracepoint (a `kprobe` cast would fail here). `load()` is the
verifier gate — if the kernel rejects the program, it's this call that
errors. `attach(category, name)` wires it to `syscalls:sys_enter_openat`.
Tracepoints need only those two strings — no symbol, no BTF — which is
the simplicity tracepoints buy you.

Reading is the same poll loop as Chapter 6, but over a `RingBuf` instead
of a per-CPU array:

```rust
while let Some(item) = ring.next() {
    let ev: OpenEvent = unsafe { core::ptr::read_unaligned(item.as_ptr() as *const OpenEvent) };
    let result = if ev.ret >= 0 { "ok" } else { "err" };
    counter.add(1, &[KeyValue::new("program", "opensnoop"), KeyValue::new("result", result)]);
    println!("{:<7} {:<6} {:<5} {:<16} {}", ev.pid, ev.uid, ev.ret,
             cstr(&ev.comm), cstr(&ev.filename));
}
```

`ring.next()` yields each submitted record as a byte view; we
`read_unaligned` it back into an `OpenEvent` (unaligned because the ring
packs records tightly, with no padding guarantees). The sign of `ret`
becomes the `result` label, so one metric series splits cleanly into
successes and failures — `ebpf_events_total{program="opensnoop",result="ok|err"}`.

## Build, deploy, observe

```bash
cd examples/09-opensnoop && ./demo.sh
```

Generate opens on the target — including a guaranteed miss so you see a
negative `ret`:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'cat /etc/hostname /etc/os-release /nope-$RANDOM 2>/dev/null; true'
```

A `PID UID RET COMM FILE` table fills, and the `result` label lets you
split opens into successes and failures in Grafana.

> **Heads up: openat is high-volume.** Almost everything opens files
> constantly. On a busy target this tool produces a *lot* of events —
> which is realistic, and a good reason later chapters add in-kernel
> filtering (by PID, by filename prefix) so user space isn't flooded.
> For now, watching the firehose makes the point that the kernel sees
> everything.

## Cross-check

```bash
[vm]$ sudo opensnoop-bpfcc
```

The BCC tool is the C/libbpf implementation of the same idea. Running
it beside your Rust `opensnoop` and comparing output is the most direct
"is mine right?" check you can do — same events, same fields, different
language.

## What you learned

- Tracepoints are stable syscall instrumentation; read their args by
  offset from the format file.
- Syscall arguments at entry are **user** memory —
  `bpf_probe_read_user_*`, not the kernel reader.
- The entry/exit `HashMap` bridge from Chapter 8 generalizes cleanly to
  tracepoints.

Next, a shorter sibling — **`sigsnoop`** — one tracepoint, signal
tracing, and friendly name mapping.

---

*Verification status: <span class="status status--unverified">unverified</span>.
The tracepoint offsets, `read_at` API, and `bpf_probe_read_user_str_bytes`
are unrun at authoring — verify the offsets against the format file
first. The first `cargo build` and `./demo.sh` on Fedora 44 are the
test.*
