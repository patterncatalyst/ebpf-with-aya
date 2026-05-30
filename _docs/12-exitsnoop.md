---
title: "exitsnoop"
order: 12
part: Tracing the kernel
description: The bookend to execsnoop — trace process termination and exit codes via the exit_group tracepoint, getting the status without touching task_struct, and learn how the exit-code encoding differs from the wait() status.
duration: 15 minutes
---

`exitsnoop` completes what `execsnoop` started: where execsnoop caught
every process launch, exitsnoop catches every termination and its exit
code. Run together they bracket the full life of every process on the
system — the foundation of process accounting and short-lived-process
detection. The interesting wrinkle here is getting the exit code
*cleanly*, and understanding why the number you read isn't encoded the
way you might expect.

The code is in `examples/12-exitsnoop/`.

## Getting the exit code without task_struct

The traditional `exitsnoop` (the libbpf/BCC one) hooks the
`sched:sched_process_exit` tracepoint and reads `exit_code` out of
`task_struct` — which means CO-RE field access and a dependency on
kernel struct layout. We take a more robust route: attach to
`syscalls:sys_enter_exit_group`.

`exit_group(2)` is the syscall every normal process termination funnels
through — glibc calls it when `main` returns or `exit()` is called, and
it terminates all threads in the process. Its single argument *is* the
exit status the program requested. So we read one tracepoint argument
and we're done — no `task_struct`, no CO-RE, robust across kernels:

```rust
#[tracepoint]
pub fn sys_enter_exit_group(ctx: TracePointContext) -> u32 {
    let code = unsafe { ctx.read_at::<i64>(16) }.unwrap_or(0) as i32;  // error_code
    if let Some(mut slot) = EVENTS.reserve::<ExitEvent>(0) {
        let ev = ExitEvent {
            pid: (bpf_get_current_pid_tgid() >> 32) as u32,
            code,
            comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    0
}
```

`ctx.read_at::<i64>(16)` reads `error_code` from the 64-bit argument
slot and narrows to `i32` (same widening rule as Chapter 10). The dying
process *is* the current process, so `bpf_get_current_pid_tgid()` and
`bpf_get_current_comm()` give us who it was — no argument needed — and
one `reserve`/`submit` ships the `ExitEvent`. As with `sigsnoop` there's
no `HashMap`: an exit is reported the instant it happens, nothing to
pair.

The trade-off: `exit_group` catches processes that exit *normally*. A
process killed by a signal (`SIGKILL`, `SIGSEGV`) never calls it, so it
won't appear here. Catching signal-deaths too means adding the
`sched:sched_process_exit` tracepoint — a reasonable extension, but it
reintroduces the `task_struct` read, so we keep the robust version as
the default and note the extension.

## The encoding gotcha

Here's the subtle part, worth pinning down because it bites everyone.
The number you read from `exit_group` is the **raw** value the program
passed: `exit(3)` arrives as `3`. The exit code is its low 8 bits:

```rust
let exit_code = ev.code & 0xff;
```

That is **not** the same encoding as `task_struct->exit_code` or the
status `wait()` hands a parent. *Those* pack the exit code into the
**high** byte and a terminating signal into the **low** byte — which is
why the libbpf exitsnoop does `(exit_code >> 8) & 0xff` for the code and
`exit_code & 0x7f` for the signal. We read the syscall argument
*before* the kernel does that packing, so for us the code is simply the
low byte. Mixing the two encodings up gives you exit codes multiplied
or divided by 256 — a classic confusing bug. The rule: *where* you read
the value determines how it's encoded.

## The user side

The attach and drain are the single-tracepoint pattern from Chapter 10 —
`program_mut("sys_enter_exit_group").try_into::<&mut TracePoint>()`,
`load()`, `attach("syscalls", "sys_enter_exit_group")`, then a
`RingBuf::try_from` loop. The only new logic is applying the decode from
the section above and turning it into a `status` label so failures pop
in Grafana:

```rust
while let Some(item) = ring.next() {
    let ev: ExitEvent = unsafe { core::ptr::read_unaligned(item.as_ptr() as *const _) };
    let exit_code = ev.code & 0xff;                       // low byte — see "encoding gotcha"
    let status = if exit_code == 0 { "ok" } else { "nonzero" };
    println!("{:<7} {:<16} {:<5} {}", ev.pid, cstr(&ev.comm), exit_code, status);
    counter.add(1, &[KeyValue::new("program", "exitsnoop"), KeyValue::new("status", status)]);
}
```

## Build, deploy, observe

```bash
cd examples/12-exitsnoop && ./demo.sh
```

Generate exits with a few different codes:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" '(true); (false); sh -c "exit 3"'
```

The `PID COMM CODE STATUS` table shows `0`, `1`, and `3`; in Grafana
the `status` label splits clean exits from failures — a fleet-wide
"what's crashing?" panel in one query.

## Cross-check

```bash
[vm]$ sudo bpftrace -e 'tracepoint:syscalls:sys_enter_exit_group { printf("%s code %d\n", comm, args.error_code); }'
```

`bpftrace` reads the same `error_code` argument, so its numbers should
match your `CODE` column directly — and confirm the `& 0xff` decode is
right.

## What you learned

- `exit_group`'s argument gives the exit code without any `task_struct`
  access — robust by construction.
- Exit-code **encoding depends on where you read it**: the syscall arg
  (low byte) differs from `task_struct->exit_code` / `wait()` status
  (code in high byte, signal in low byte).
- execsnoop + exitsnoop bracket the process lifecycle.

That closes **Tracing the kernel**. The next part, *User-space &
language probing*, turns the lens around — uprobes, USDT, and probing
inside running applications, starting with `bashreadline`. See the
[roadmap]({{ "/plans/iteration-plan/" | relative_url }}).

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm the `error_code` offset, the `read_at`/attach API, and the
`& 0xff` decode against a known `exit(N)` on Fedora 44. The first build
and run are the test.*
