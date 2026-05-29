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
    let target_pid = ctx.read_at::<i64>(16)? as i32;
    let sig        = ctx.read_at::<i64>(24)? as i32;
    // + current pid/comm, then submit one SignalEvent
    0
}
```

That's the entire probe. No maps to bridge, no pointers to chase — the
floor of what a useful eBPF tracer looks like.

## Keep the kernel dumb; format in user space

The kernel program records the *number* `15`, not the string
`"SIGTERM"`. Mapping numbers to names, looking up process names,
pretty-printing — all of that happens in user space, where you have an
allocator, the standard library, and no verifier. This is a deliberate
pattern: the in-kernel half should do the minimum (capture raw facts on
the hot path), and the user half does everything else.

`sigsnoop/src/main.rs` maps the common signals and labels the metric by
name:

```rust
fn sig_name(sig: i32) -> &'static str {
    match sig { 2 => "SIGINT", 9 => "SIGKILL", 15 => "SIGTERM", /* ... */ _ => "SIG?" }
}
// ...
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
process lifecycle with **`execsnoop`** and **`exitsnoop`**. See the
[roadmap]({{ "/plans/iteration-plan/" | relative_url }}).

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm the `sys_enter_kill` offsets against the format file and the
`read_at`/attach API on Fedora 44. The first build and run are the
test.*
