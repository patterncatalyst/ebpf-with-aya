---
title: "Syscall programs: when BPF loads BPF"
order: 49
part: Advanced kernel surface
description: "A program type with no hook: BPF_PROG_TYPE_SYSCALL programs run on demand and can call the bpf() syscall themselves via bpf_sys_bpf. Understand loader programs — a 'strace of libbpf' replayed in-kernel — and the light skeletons bpftool builds from them, the path toward libbpf-free and signed program loading."
duration: 35 minutes
---

Every program so far attaches to *something* — a kprobe, a tracepoint, a
socket, a scheduler slot. **`BPF_PROG_TYPE_SYSCALL`** is the odd one out: it
attaches to nothing. You *run* it, once, on demand, and while it runs it can
issue **`bpf()` syscalls itself** through the `bpf_sys_bpf()` helper. A BPF
program that loads BPF. This chapter explains why that exists, what a *loader
program* is, and how it underpins the **light skeletons** you've probably seen
`bpftool` generate — the road toward loading eBPF without libbpf, and toward
*signed* programs.

This is a concept chapter with an inspectable artifact rather than a new Aya
tool — see `examples/49-syscall-prog/` and its `README.md`. The reason is
worth stating plainly: **Aya is itself the user-space loader.** The syscall
program type is libbpf/light-skeleton machinery, so you'll usually *meet* it,
not write it, and the chapter reflects that.

{% include excalidraw.html
   file="syscall-prog"
   alt="Two ways to create BPF objects. The usual way: user space (libbpf or aya) issues many bpf() syscalls — map create, prog load — to build maps and programs in the kernel. The loader-program way: user space runs a syscall program once via BPF_PROG_RUN, and that program calls bpf_sys_bpf N times in the kernel to create the same maps and programs. Running once replays the load sequence in-kernel, which is the basis of light skeletons and signed loading."
   caption="Figure 49.1 — A loader program issues the bpf() commands itself, in-kernel" %}

## The usual way, and its cost

When any loader — libbpf, or Aya — brings up a BPF object file, it performs a
*sequence* of `bpf()` syscalls: load BTF, create each map, populate read-only
data, then load each program with its relocations applied. To do that, the
loader must parse ELF, understand BTF, and carry a lot of code: libbpf plus
libelf, or Aya's loading machinery. That's a real dependency for anything that
wants to ship a single small binary, run in a constrained init, or be embedded
in a kernel module.

## The loader program

The syscall program type inverts this. Instead of *performing* the load
sequence from user space, you capture it as a **loader program**: a single BPF
program — of type `BPF_PROG_TYPE_SYSCALL` — whose body is essentially "a strace
of libbpf." When executed via **`BPF_PROG_RUN`** (the same test-run mechanism
from Chapter 35, here used to *do work* rather than test), it calls
`bpf_sys_bpf()` for each step — create this map, load that program — and ends
with the same maps and programs created as if libbpf had done it. A couple of
properties fall out of running inside the kernel:

- It must be **sleepable**, because populating read-only data uses
  `bpf_copy_from_user`.
- It works through a small set of syscall-context helpers: `bpf_sys_bpf(cmd,
  attr, size)` to issue a `bpf()` command, `bpf_sys_close(fd)` to clean up
  intermediate file descriptors, and `bpf_btf_find_by_name_kind()` for BTF
  lookups.

Illustratively, the body looks like a series of `bpf_sys_bpf` calls (this is a
sketch — syscall-program support in aya-ebpf is experimental, and you'd
normally generate, not hand-write, this):

```rust
// ILLUSTRATIVE — a loader program's shape, not a turnkey Aya program.
fn loader(ctx: &SyscallContext) -> i64 {
    // 1. create a map
    let mut attr = bpf_attr_map_create { map_type: HASH, key_size: 4, value_size: 8, max_entries: 1, .. };
    let map_fd = bpf_sys_bpf(BPF_MAP_CREATE, &mut attr, size_of_val(&attr));
    // 2. load a program that uses it (instructions embedded as data)
    let mut load = bpf_attr_prog_load { prog_type: TRACEPOINT, insns: .., insn_cnt: .., map_fds: [map_fd], .. };
    let prog_fd = bpf_sys_bpf(BPF_PROG_LOAD, &mut load, size_of_val(&load));
    bpf_sys_close(map_fd);
    prog_fd
}
```

The instructions, map definitions, and initial values the loader needs are
embedded as data alongside it. Run it once and the kernel has everything.

## Light skeletons

This is not a thing you typically write by hand — `bpftool` generates it. Take
any compiled BPF object and ask for a *light* skeleton:

```bash
[vm]$ bpftool gen skeleton -L program.o > program.lskel.h
```

The normal `gen skeleton` embeds the whole ELF and relies on libbpf to parse
it later. The **light** skeleton (`-L`) instead embeds a generated
**`BPF_PROG_TYPE_SYSCALL` loader program** plus the data it needs, and exposes
open/load/attach functions that only need a few thin `bpf()` wrappers — **no
libelf, no libbpf** at load time. That's how BPF programs get embedded in
places that can't carry libbpf: tiny static binaries, early init, even kernel
modules.

And because the entire load is now *one program plus its data*, it becomes a
single object you can **sign** and have the kernel verify before it runs — the
long-stated goal this machinery was built toward, and a real ingredient of
trusted BPF deployment.

## Where Aya fits

Be clear-eyed about this one: Aya already does in user space, robustly, what a
loader program does in the kernel — it *is* the loader. So you won't reach for
`BPF_PROG_TYPE_SYSCALL` to replace your Aya loader. It matters for three
reasons, all of which this chapter is really about: understanding what a
**light skeleton** contains when you see one; understanding the path to
**signed** program loading; and recognizing the broader idea that a BPF
program can drive the `bpf()` API itself — useful for niche tasks like batched
fd cleanup, not just loading. Writing these in aya-ebpf is experimental
territory today.

## Build, deploy, observe

```bash
cd examples/49-syscall-prog && ./demo.sh
```

The demo takes a BPF object built by an earlier example and runs `bpftool gen
skeleton -L` on it, so you can *see* the generated syscall/loader program and
its embedded data in the light skeleton — the abstract idea made concrete. The
`illustrative/` directory holds the loader-program sketch above for reading.

## Cross-check

```bash
[vm]$ bpftool gen skeleton -L program.o | sed -n '1,40p'   # the loader program + data
[vm]$ bpftool prog show | grep -i syscall                  # a loaded syscall prog, if any
[vm]$ bpftool prog load program.o /sys/fs/bpf/p autoattach  # the ordinary path, for contrast
```

Reading the light skeleton's embedded byte arrays and the `*_load` function it
generates is the cross-check: you can see the load sequence has become a
program plus data, with libbpf no longer in the loading path.

## What you learned

- **`BPF_PROG_TYPE_SYSCALL`** programs attach to nothing; you run them via
  `BPF_PROG_RUN`, and they can call `bpf()` themselves with `bpf_sys_bpf()`.
- A **loader program** captures libbpf's load sequence as one sleepable BPF
  program; running it recreates the maps and programs in-kernel, removing
  libelf/libbpf from the loading path.
- That program plus its data is what `bpftool gen skeleton -L` embeds in a
  **light skeleton**, and being a single object is what makes **signed**
  loading possible. Aya is itself the loader, so this is mechanism to
  recognize more than a tool to wield.

Next, Chapter 50 looks at the **user ring buffer** — a channel that runs the
other direction, from user space into a BPF program.

---

*Verification status: <span class="status status--verified">verified</span>
— Fedora 44, kernel 7.1.3 (bpftool v7.6.0, libbpf 1.6). `bpftool gen skeleton
-L` on a libbpf-style C object (`reference/skel_demo.bpf.c`, compiled on the
target) emits the `struct bpf_loader_ctx` / `skel_internal.h` light skeleton —
the BPF_PROG_TYPE_SYSCALL loader path — while the full skeleton emits the
classic `libbpf.h` / `bpf_object_skeleton` ELF path. Note: aya-ebpf objects
can't be used here — they carry legacy `maps`-section definitions that libbpf
v1.0+ rejects. The aya-ebpf loader-program sketch stays illustrative —
syscall-program authoring in aya is experimental.*
