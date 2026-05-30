---
title: "execsnoop"
order: 11
part: Tracing the kernel
description: Trace every program launch with its full command line — the new skill is reading argv, an array of user-space string pointers, in a bounded verifier-friendly loop, written straight into the ring-buffer slot.
duration: 30 minutes
---

`execsnoop` shows every program that runs on the system, with its
command line. It's one of the most useful tools in the eBPF canon —
short-lived processes that `ps` and `top` never catch show up here the
instant they `exec`. The new skill this chapter teaches is reading
**argv**: not a single string like the filename in `opensnoop`, but an
*array* of user-space string pointers, read in a loop the verifier will
accept.

The code is in `examples/11-execsnoop/`.

{% include excalidraw.html
   file="tracepoint-flow"
   alt="How a trace works: a process calls execve, a tracepoint fires, the eBPF program reads the event's fields by offset (and argv in a bounded loop), and ships the result to user space via a ring buffer."
   caption="Figure 11.1 — anatomy of a trace" %}

## The challenge: argv is an array of pointers

`opensnoop` read one user string (the filename). `execve`'s second
argument, `argv`, is harder: it's a `const char *const *` — a user
pointer to an *array* of user pointers, each pointing to a string. To
capture the command line you must:

1. read the `argv` pointer from the tracepoint record;
2. loop: read the *i*-th pointer out of that user array;
3. if it's null, stop (that's the end of argv);
4. otherwise read the string it points to.

Two eBPF constraints shape how we write that loop.

**The verifier needs a constant bound.** You cannot loop "until null"
unboundedly — the verifier must prove termination. So we loop a fixed
`MAX_ARGS` times and break early on null. We use `MAX_ARGS = 8`,
`ARG_LEN = 64`: generous enough to be useful, small enough to stay
cheap.

**The event is too big for the stack.** A BPF program's stack is 512
bytes. Our `ExecEvent` — filename plus eight 64-byte arg slots — is
~800 bytes. So we **reserve the ring-buffer slot first and write
directly into it**, never materializing the event on the stack:

```rust
let mut slot = EVENTS.reserve::<ExecEvent>(0)?;
let ev = slot.as_mut_ptr();           // write through this pointer
```

## Reading argv into fixed slots

Storing args as `args: [[u8; ARG_LEN]; MAX_ARGS]` — a fixed grid — is a
deliberate choice. The obvious alternative, packing args end-to-end
into one buffer with a running offset, requires *dynamic* indexing
(`buf[offset..]` where `offset` varies), which forces you into the
verifier's bounded-pointer-arithmetic rules and a lot of masking. Fixed
slots indexed by the loop counter sidestep all of that: `args[i]` with
`i` bounded by a constant loop is trivially safe. The loop:

```rust
let argv: *const *const u8 = ctx.read_at(ARGV_OFF)?;
let mut count = 0;
for i in 0..MAX_ARGS {
    let argp = match bpf_probe_read_user::<*const u8>(argv.add(i)) {
        Ok(p) => p, Err(_) => break,
    };
    if argp.is_null() { break; }
    bpf_probe_read_user_str_bytes(argp, &mut (*ev).args[i]);
    count += 1;
}
(*ev).args_count = count;
```

`bpf_probe_read_user::<*const u8>(argv.add(i))` reads one pointer-sized
value out of the user array; `bpf_probe_read_user_str_bytes` then copies
the string it points at. Both are the *user* readers — argv and its
strings all live in the calling process's memory at this point, the
same lesson from `opensnoop`.

> **This loop is the part most likely to need a tweak.** Reading an
> array of user pointers is exactly where the verifier gets picky, and
> the acceptable form shifts a little between kernel and aya versions.
> If `cargo build`'s verifier step rejects it, compare against the
> execsnoop in the Aya examples repo and adjust — the *shape* here is
> right; a bound or a cast may need nudging. It's flagged in the
> reconciliation plan.

## The user side

Attaching is the now-familiar tracepoint pattern
(`syscalls:sys_enter_execve`). The only new user-space work is
reassembling the command line from the fixed slots:

```rust
fn cmdline(ev: &ExecEvent) -> String {
    (0..ev.args_count.min(MAX_ARGS as u32) as usize)
        .map(|i| cstr(&ev.args[i]))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
```

Dumb kernel, smart user space again: the kernel captured raw slots; the
prettifying is up here.

## Build, deploy, observe

```bash
cd examples/11-execsnoop && ./demo.sh
```

Run some commands on the target:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'ls -la /tmp; uname -a; id'
```

The `PID UID COMM CMDLINE` table shows each command with its arguments
reconstructed — `ls -la /tmp`, not just `ls`. That argument visibility
is exactly what makes execsnoop a security favourite: a suspicious
`curl http://…​/x.sh | sh` is obvious in the cmdline and invisible in a
bare process name.

## Cross-check

```bash
[vm]$ sudo execsnoop-bpfcc
```

The BCC tool prints the same launches. If your `CMDLINE` column matches
its `ARGS` column, your argv loop is correct — the single most valuable
check for this chapter, since the argv loop is the tricky part.

## What you learned

- Reading argv: a bounded loop over an array of user pointers, into
  fixed slots to keep the verifier happy.
- Large events go **directly into the reserved ring slot**, never the
  512-byte stack.
- Command-line visibility is what makes execve tracing powerful.

Next, the bookend: **`exitsnoop`** — process termination and exit
codes, completing the lifecycle. See the
[roadmap]({{ "/plans/iteration-plan/" | relative_url }}).

---

*Verification status: <span class="status status--unverified">unverified</span>.
The argv loop, `bpf_probe_read_user` signatures, the execve offsets, and
the large-event `reserve` are unrun at authoring — the argv loop is the
highest-risk item. The first `cargo build` and `./demo.sh` on Fedora 44
are the test.*
