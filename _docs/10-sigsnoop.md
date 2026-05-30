---
title: "sigsnoop"
order: 10
part: Tracing the kernel
description: A shorter sibling to opensnoop — one tracepoint on the kill syscall to trace which process signals which, with signal-number-to-name mapping done in user space.
duration: 15 minutes
---

`sigsnoop` is the minimal per-event tool: a single tracepoint, one
event per signal sent, no entry/exit correlation. It traces `kill(2)` —
who sent which signal to whom — and is a good place to reinforce two
ideas with very little code: reading a couple of tracepoint arguments,
and keeping the kernel program dumb while user space does the friendly
formatting.

The code is in `examples/10-sigsnoop/`.

## One tracepoint, one event

Where `opensnoop` paired enter and exit, `sigsnoop` needs only the
*enter* of `kill`: the target PID and the signal number are both
arguments, and the sender is the current process. So the whole kernel
program is: read two args, add the current pid/comm, emit one event.

From the format file (`/sys/kernel/tracing/events/syscalls/sys_enter_kill/format`):
`pid`@16, `sig`@24.

```rust
#[tracepoint]
pub fn sys_enter_kill(ctx: TracePointContext) -> u32 {
    let target_pid = unsafe { ctx.read_at::<i64>(16) }.unwrap_or(0) as i32;
    let sig        = unsafe { ctx.read_at::<i64>(24) }.unwrap_or(0) as i32;

    if let Some(mut slot) = EVENTS.reserve::<SignalEvent>(0) {
        let ev = SignalEvent {
            sender_pid: (bpf_get_current_pid_tgid() >> 32) as u32,
            target_pid, sig,
            comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    0
}
```

Reading it as Aya sees it: `ctx.read_at::<i64>(16)` pulls the `pid`
argument from the tracepoint record at the offset the format file gave
us. Syscall tracepoints store *every* argument in a 64-bit slot
regardless of its C type, so we read an `i64` and narrow to `i32` — ask
for an `i32` directly and you'd take four bytes of an eight-byte slot
and get away with it on little-endian x86_64 until the day you don't.
The sender needs no argument: it's the current process, so
`bpf_get_current_pid_tgid() >> 32` is its PID and
`bpf_get_current_comm()` its name. Then the same `reserve` → write →
`submit` ring-buffer emit from Chapter 9, sized for one `SignalEvent`.
There's no `HashMap` here because there's nothing to correlate — `kill`
is reported the instant it's called, so this is the floor of what a
useful tracer looks like.

## The user side

A lone tracepoint is the simplest possible attach — one program, one
`(category, name)` pair, then drain:

```rust
let prog: &mut TracePoint = ebpf.program_mut("sys_enter_kill").unwrap().try_into()?;
prog.load()?;
prog.attach("syscalls", "sys_enter_kill")?;
let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;
```

`try_into::<&mut TracePoint>()` confirms the program is a tracepoint,
`load()` runs the verifier, and `attach("syscalls", "sys_enter_kill")`
wires it up. Draining is the Chapter 9 loop — `ring.next()`,
`read_unaligned` into a `SignalEvent` — with the one twist this chapter
is about: the kernel handed us the signal *number*, and turning that
into something legible is user space's job.

## Keep the kernel dumb; format in user space

The kernel program records the *number* `15`, not the string
`"SIGTERM"`. Mapping numbers to names, looking up process names,
pretty-printing — all of that happens in user space, where you have an
allocator, the standard library, and no verifier. This is a deliberate
pattern: the in-kernel half should do the minimum (capture raw facts on
the hot path), and the user half does everything else.

In the drain loop, `sigsnoop/src/main.rs` maps the number to a name and
uses it both for display and as a metric label:

```rust
fn sig_name(sig: i32) -> &'static str {
    match sig { 2 => "SIGINT", 9 => "SIGKILL", 15 => "SIGTERM", /* ... */ _ => "SIG?" }
}
// inside `while let Some(item) = ring.next()`:
let ev: SignalEvent = unsafe { core::ptr::read_unaligned(item.as_ptr() as *const _) };
let name = sig_name(ev.sig);
println!("{:<7} {:<16} {:<8} {}", ev.sender_pid, cstr(&ev.comm), name, ev.target_pid);
counter.add(1, &[KeyValue::new("program", "sigsnoop"), KeyValue::new("signal", name)]);
```

Now Grafana can break `ebpf_events_total` down by signal type — a
`SIGKILL` spike from one process is the kind of thing that's invisible
in aggregate but obvious once labelled.

## Build, deploy, observe

```bash
cd examples/10-sigsnoop && ./demo.sh
```

Send a signal on the target:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'sleep 60 & p=$!; kill -TERM $p; true'
```

The `SENDER COMM SIGNAL TARGET` table shows the `SIGTERM`, and the
`signal` label appears in Grafana.

## Cross-check

```bash
[vm]$ sudo bpftrace -e 'tracepoint:syscalls:sys_enter_kill { printf("%s -> %d sig %d\n", comm, args.pid, args.sig); }'
```

Note `bpftrace` reads the same args by *name* (`args.pid`, `args.sig`)
because it resolves the format file for you; your Aya program reads them
by *offset*. Same data, two ways of naming it — and a good reminder that
the offsets you hardcoded come straight from that format.

## What you learned

- The minimal tracer: one tracepoint, two arg reads, one event.
- The division of labor — dumb kernel program, smart user space —
  applied to signal-name mapping and metric labelling.

This closes the first sweep of "Tracing the kernel." Next we turn to
process lifecycle with **`execsnoop`** and **`exitsnoop`**.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm the `sys_enter_kill` offsets against the format file and the
`read_at`/attach API on Fedora 44. The first build and run are the
test.*
