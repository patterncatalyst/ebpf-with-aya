---
title: "Protecting a file with LSM"
order: 40
part: Security & LSM
description: "The defensive counterpart to the offense chapter: an LSM program on inode_permission that denies write access to one protected file by inode — making it read-only even for root — and a first look at reading a kernel struct field from an LSM program (with CO-RE deferred to Part 9)."
duration: 40 minutes
---

Chapter 39 showed how eBPF can tamper with what userspace sees. This chapter
uses the same hook family for defense: make a specific file **unmodifiable**,
even by root. We attach to the LSM hook that mediates inode access,
`inode_permission`, and deny any write to one protected inode. It's a small
program, but it introduces the skill the rest of the security and tracing
worlds lean on — **reading a kernel struct field from inside a BPF program**
— and shows plainly where that gets fragile and how Part 9's CO-RE fixes it.

The code is in `examples/40-lsm-fileprotect/`. `./demo.sh` there builds,
deploys, and runs it; its `README.md` covers what it does and how to drive
it.

{% include excalidraw.html
   file="lsm-file-protect"
   alt="A write or truncate of a protected file reaches the LSM inode_permission hook, which checks whether the inode's i_ino equals the protected inode. If the access requests MAY_WRITE on the protected inode, the program returns -EPERM and the change is blocked; reads and accesses to other files return 0 and proceed, so the file stays readable. This is the defensive counterpart to Chapter 39; portable field reads come with CO-RE in Part 9."
   caption="Figure 40.1 — Deny writes to one inode, even for root" %}

## The hook and the field

`inode_permission(struct inode *inode, int mask)` fires on the kernel's
access check for an inode — on opens, writes, truncates, and more. The
`mask` says what's being attempted (`MAY_READ`, `MAY_WRITE`, `MAY_EXEC`,
`MAY_APPEND`), and the `inode` is the file in question. To protect one file
we need to recognize it, and the stable identity of a file is its **inode
number**, `inode->i_ino`. So the program reads one field off a kernel
pointer — the new skill here.

Chapter 37's program never touched a kernel struct; it decided on a cgroup
id from a helper. Here we must dereference `inode` to get `i_ino`. In an LSM
program that pointer is *trusted* (the verifier knows its BTF type), so in
principle you read the field directly. In practice that needs the kernel's
type definitions compiled in — which is the CO-RE machinery this book builds
properly in Part 9. Until then we read the field the explicit way, with
`bpf_probe_read_kernel` at the field's offset, and we're upfront that the
offset is kernel-version-specific.

## How the code works

```rust
#[map] static PROTECTED: Array<u64> = Array::with_max_entries(1, 0); // protected inode number
#[map] static DENIED:    HashMap<u32, u64> = HashMap::with_max_entries(1, 0);

const MAY_WRITE: i32 = 0x02;
// Offset of i_ino within struct inode on the target kernel. VERSION-SPECIFIC —
// CO-RE (Part 9) computes this at load time instead of hard-coding it.
const I_INO_OFFSET: usize = 40;

#[lsm(hook = "inode_permission")]
pub fn protect_file(ctx: LsmContext) -> i32 {
    // inode_permission args: (inode, mask, ret)
    let prior: i32 = unsafe { ctx.arg(2) };
    if prior != 0 { return prior; }

    let inode: *const u8 = unsafe { ctx.arg(0) };
    let mask: i32 = unsafe { ctx.arg(1) };
    if mask & MAY_WRITE == 0 { return 0; }            // only police writes

    let i_ino: u64 = match unsafe { bpf_probe_read_kernel((inode.add(I_INO_OFFSET)) as *const u64) } {
        Ok(v) => v,
        Err(_) => return 0,                           // can't read → don't break the system
    };
    let protected = unsafe { PROTECTED.get(0).copied() }.unwrap_or(0);
    if protected != 0 && i_ino == protected {
        bump(&DENIED, 0, 1);
        return -1;                                    // -EPERM
    }
    0
}
```

Reading it the way you'd write it:

- **Respect the prior verdict** (arg index 2, since `inode_permission` has
  two real args) — same chain etiquette as Chapter 37.
- **Filter to writes first.** `inode_permission` fires constantly, including
  on every read; checking `mask & MAY_WRITE` early means we only do work
  (and only ever deny) for write attempts, so reads of the protected file —
  and all access to every other file — sail through with `0`.
- **Read `i_ino`.** `bpf_probe_read_kernel` copies the 8-byte inode number
  from `inode + I_INO_OFFSET`. If the read fails we **fail open** (return
  `0`): a security program that returns errors on every inode access would
  wedge the machine, so when unsure we allow.
- **Decide.** If the inode matches the protected one, return `-1`
  (`-EPERM`); the write is refused with "Operation not permitted," even for
  root. Otherwise allow.

The hard-coded `I_INO_OFFSET` is the real weak point. It's correct only
for a particular kernel build; on another kernel the program would read the
wrong bytes and either protect nothing or the wrong file. That's precisely
the portability problem **CO-RE** solves — the loader rewrites such offsets
from the running kernel's BTF at load time — and it's a big enough topic to
get its own treatment in Part 9. For now, treat the offset as a knob you
confirm against your kernel.

### Loading and setting the target

```rust
let protected_ino = std::fs::metadata(&path)?.ino();          // the file to protect
{
    let mut p: Array<_, u64> = Array::try_from(ebpf.map_mut("PROTECTED").unwrap())?;
    p.set(0, protected_ino, 0)?;
}
let btf = Btf::from_sys_fs()?;
let prog: &mut Lsm = ebpf.program_mut("protect_file").unwrap().try_into()?;
prog.load("inode_permission", &btf)?;
prog.attach()?;
```

The loader `stat`s the file for its inode number, writes it into
`PROTECTED`, loads against BTF, and attaches — then drains `DENIED` into
`ebpf_lsm_denied_total`.

## Build, deploy, observe

```bash
cd examples/40-lsm-fileprotect && ./demo.sh
```

The demo creates `/tmp/ebpf-protected` with some content, protects it, then
loops: it reads the file (works), and tries to append to it (fails with
"Operation not permitted"). `ebpf_lsm_denied_total` climbs with each blocked
write; detach and the file is writable again.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `rate(ebpf_lsm_denied_total[1m])` — blocked writes to the protected file (filter to the `ebpf-lsm-fileprotect` service — it shares the metric name with Chapter 37).

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`):

```bash
[vm]$ stat -c '%i %n' /tmp/ebpf-protected      # the inode we protected
[vm]$ cat /tmp/ebpf-protected                  # reads still work
[vm]$ echo tamper >> /tmp/ebpf-protected
bash: /tmp/ebpf-protected: Operation not permitted
[vm]$ sudo sh -c 'echo tamper >> /tmp/ebpf-protected'
sh: /tmp/ebpf-protected: Operation not permitted     # even as root
```

A write refused with `EPERM` while `cat` still works — and the refusal
holding even under `sudo` — is MAC doing what discretionary file
permissions can't.

## What you learned

- An LSM program on **`inode_permission`** can make a file **read-only even
  to root** by denying `MAY_WRITE` on its inode — the defensive use of the
  hook family Chapter 39 abused.
- **Reading a kernel struct field** (`i_ino` via `bpf_probe_read_kernel`),
  filtering early on `mask`, and the **fail-open** rule for security
  programs (allow when a read fails, so a bug can't wedge the box).
- Why the hard-coded offset is a liability and that **CO-RE (Part 9)** is the
  portable fix.

That rounds out the introduction to security with eBPF — confining,
reacting, hiding, and protecting. Later parts return to this surface with
richer kernel-struct access once CO-RE is in hand.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
