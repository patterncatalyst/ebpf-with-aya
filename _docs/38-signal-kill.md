---
title: "Signal programs: reacting with force"
order: 38
part: Security & LSM
description: "The other enforcement style: instead of denying a syscall at an LSM hook, react to one. A tracepoint on execve matches a forbidden binary and calls bpf_send_signal(SIGKILL) to kill the process before it runs — a 'signal program', lab-only offense that illuminates defense."
duration: 40 minutes
---

Chapter 37 *prevented* an operation by returning `-EPERM` from an LSM hook.
There's a second enforcement style: let the event happen and **react** to
it. eBPF can send a signal to a process from inside the kernel with
`bpf_send_signal()`, so a program can watch for something it doesn't like
and **kill the offender on the spot**. This chapter builds the canonical
example — kill any process that tries to execute a forbidden binary, before
it gets to run — which doubles as a lesson in why this technique is powerful
*and* why it belongs in a lab.

The code is in `examples/38-signal-kill/`. `./demo.sh` there builds,
deploys, and runs it; its `README.md` covers what it does and how to drive
it.

{% include excalidraw.html
   file="signal-kill"
   alt="A process calls execve on /tmp/forbidden-something. A tracepoint on sys_enter_execve reads the filename; if it matches the forbidden prefix, the program calls bpf_send_signal(SIGKILL) and the process dies before the exec completes. React, don't just watch — a signal program is lab-only offense that illuminates defense."
   caption="Figure 38.1 — React, don't just watch: kill the process the moment it execs a forbidden binary" %}

## Prevent vs. react

Two ways to stop something with eBPF:

- **Prevent** (Chapter 37): an LSM hook returns an error and the syscall
  never succeeds. Clean, but only at hooks the kernel offers, and only with
  an allow/deny answer.
- **React**: watch any event you can probe, and if it's bad, *do something*
  — emit an alert, or `bpf_send_signal()` to kill the process. Broader reach
  (any tracepoint/kprobe), but it's after-the-fact: you're racing the
  action, and a signal is a blunt instrument.

`bpf_send_signal(sig)` sends a signal to the *current* task — the one whose
event you're handling. Catch a process at `sys_enter_execve` and send
`SIGKILL`, and it dies before the new program image is running. That's the
"kill on sight" pattern: a denylist enforced with lethal force.

A word on framing: this is **lab-only**. Killing processes from the kernel
based on a string match is a great way to understand how EDR and runtime
security tools work — and a great way to make a real system unusable. We
build it to learn the mechanism and its hazards, not to deploy it.

## How the code works

### Matching and killing

```rust
const NEEDLE: &[u8] = b"/tmp/forbidden";   // kill anything exec'd from this prefix
const SIGKILL: u32 = 9;

#[tracepoint]
pub fn kill_on_exec(ctx: TracePointContext) -> u32 {
    let _ = handle(&ctx);
    0
}
fn handle(ctx: &TracePointContext) -> Result<(), ()> {
    // sys_enter_execve: the filename pointer is at offset 16 (as in Ch 11)
    let fname: *const u8 = unsafe { ctx.read_at(16) }.map_err(|_| ())?;
    let mut buf = [0u8; 64];
    let _ = unsafe { bpf_probe_read_user_str_bytes(fname, &mut buf) };

    if starts_with(&buf, NEEDLE) {
        let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
        emit_kill(pid);                       // tell user space who we killed
        unsafe { bpf_send_signal(SIGKILL); }  // and kill it
    }
    Ok(())
}

#[inline(always)]
fn starts_with(buf: &[u8; 64], needle: &[u8]) -> bool {
    let mut i = 0;
    while i < needle.len() {                  // bounded by a const length → verifier-OK
        if buf[i] != needle[i] { return false; }
        i += 1;
    }
    true
}
```

The pieces, the way you'd write them:

- We hook **`sys_enter_execve`** exactly as `execsnoop` did in Chapter 11,
  reading the filename pointer from the tracepoint record at offset 16 and
  copying the string with `bpf_probe_read_user_str_bytes` (it's a
  *user-space* pointer, so the user reader).
- **`starts_with`** compares the filename against a fixed prefix. The loop
  is bounded by `needle.len()`, a compile-time constant, and `buf` is
  indexed within its fixed 64-byte size — both required for the verifier to
  accept it. String matching in eBPF lives or dies by these bounds.
- On a match we **`bpf_send_signal(SIGKILL)`**. Because the program runs in
  the context of the process calling `execve`, the signal targets that
  process; it's killed before the new image takes over. We also emit a small
  record first so user space can report *what* was killed.

### Reporting what died

A `RingBuf` carries one record per kill so the loader can show it:

```rust
#[map] static KILLS: RingBuf = RingBuf::with_byte_size(64 * 1024, 0);

fn emit_kill(pid: u32) {
    if let Some(mut slot) = KILLS.reserve::<KillEvent>(0) {
        let ev = KillEvent { pid, comm: bpf_get_current_comm().unwrap_or_default() };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
}
```

User space attaches the tracepoint, drains the ring, and for each record
prints `killed <comm> (pid N)` and bumps `ebpf_signal_kills_total` — the
same attach/drain pattern as every tracing chapter, with a lethal side
effect in the kernel.

## Build, deploy, observe

```bash
cd examples/38-signal-kill && ./demo.sh
```

The demo loads the program on the target, then in a loop copies a harmless
binary to `/tmp/forbidden-sleep` and tries to run it — each attempt is
killed instantly — while also running a normal `sleep` that survives. You'll
see `killed forbidden-sleep (pid …)` lines from the loader and
`ebpf_signal_kills_total` rising in Grafana, and on the target the forbidden
command exits immediately with "Killed".

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`):

```bash
[vm]$ cp /usr/bin/sleep /tmp/forbidden-sleep
[vm]$ /tmp/forbidden-sleep 60 ; echo "exit=$?"   # killed: exit shows the signal
Killed
exit=137                                          # 128 + 9 (SIGKILL)
[vm]$ sleep 60 &                                  # an allowed binary keeps running
```

Exit status `137` (128 + SIGKILL's 9) on the forbidden binary while a normal
`sleep` runs untouched is the program working; `dmesg`/`journalctl` will also
show the process terminated by `SIGKILL`.

## What you learned

- **`bpf_send_signal()`** lets a kernel-side eBPF program kill (or signal)
  the current process — the *react* enforcement style, complementing
  Chapter 37's *prevent*.
- Safe **string matching** in eBPF: copy into a fixed buffer and compare
  against a constant-length needle so every access is bounded for the
  verifier.
- Why "kill on sight" is **lab-only**: it's the mechanism behind runtime
  security tooling, and also a fast way to break a real system — learn it,
  don't ship it as written.

Chapter 39 continues the Security part, building toward richer LSM policy
and tamper detection.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that `bpf_send_signal` is callable from a
syscall-entry tracepoint and kills the caller before `execve` completes, the
`sys_enter_execve` filename offset (16) as used in Chapter 11, the bounded
`starts_with` passing the verifier, and that the killed process reports exit
137.*
