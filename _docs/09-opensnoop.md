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

The code is in `examples/09-opensnoop/`.

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
— which is why the chapter has you read it. That's the honest
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

## Entry and exit, paired

The kernel side (`opensnoop-ebpf/src/main.rs`) mirrors Chapter 8's
structure, just with tracepoints:

```rust
#[tracepoint]
pub fn sys_enter_openat(ctx: TracePointContext) -> u32 {
    // read filename (user mem) + flags; store in INFLIGHT[pid_tgid]
    0
}

#[tracepoint]
pub fn sys_exit_openat(ctx: TracePointContext) -> u32 {
    // read ret@16; look up INFLIGHT; emit completed event; clear
    0
}
```

A return value `>= 0` is the new file descriptor; a negative value is
`-errno` (e.g. `-2` = `ENOENT`). User space turns that into an
`ok`/`err` label on the metric.

## The user side

Attaching tracepoints needs only their category and name — no BTF, no
function symbol:

```rust
let enter: &mut TracePoint = ebpf.program_mut("sys_enter_openat").unwrap().try_into()?;
enter.load()?;
enter.attach("syscalls", "sys_enter_openat")?;
```

Draining the ring buffer and exporting is identical to the previous
chapters. The metric is
`ebpf_events_total{program="opensnoop",result="ok|err"}`.

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
tracing, and friendly name mapping. See the
[roadmap]({{ "/plans/iteration-plan/" | relative_url }}).

---

*Verification status: <span class="status status--unverified">unverified</span>.
The tracepoint offsets, `read_at` API, and `bpf_probe_read_user_str_bytes`
are unrun at authoring — verify the offsets against the format file
first. The first `cargo build` and `./demo.sh` on Fedora 44 are the
test.*
