---
title: "CO-RE: compile once, run everywhere"
order: 58
part: Operating eBPF
description: "The portability machinery the whole book has leaned on, finally examined head-on. Kernel struct layouts differ across versions, so a hardcoded field offset breaks on the next kernel. CO-RE compiles your program once against generic types, records relocations that name the fields you touch, and patches them at load time against the target kernel's BTF. Understand the relocations, how Aya does it transparently, how aya-tool generates portable bindings, and what 'works across the fleet' actually requires."
duration: 45 minutes
---

Part 9 is about *operating* eBPF — running it across a fleet, not just on the
one VM in front of you — and that starts with the question every chapter so far
has quietly relied on someone else answering: **why does a program that reads
`task->pid` on your laptop's kernel still read the right field on a server
running a different one?** Struct layouts change between kernel versions; a
field can move, gain neighbours, or slip into a nested anonymous union. A
program compiled with a fixed offset would read garbage on the next kernel. The
answer is **CO-RE — Compile Once, Run Everywhere** — and it's been working
invisibly under Chapter 15's BTF uprobe, Chapter 52's `task_struct` kfunc
arguments, and every field read since. Time to see the machinery.

The code is in `examples/58-core/`. `./demo.sh` reads task fields portably,
handles a field that may not exist on every kernel, and shows the relocations
in the object; the `README.md` has the details.

{% include excalidraw.html
   file="core"
   alt="Compile once against generic types; relocate to the target kernel at load. In the development phase, eBPF source that reads task->pid plus a vmlinux.h generated from BTF go through the compiler (preserve_access_index), which emits a .o with BTF relocations describing 'field pid in task_struct' by name, not offset. At target runtime, you ship that one .o; the loader (aya or libbpf) reads the target kernel BTF at /sys/kernel/btf/vmlinux and patches offsets by name, producing a program loaded and running with the correct offsets for this kernel. Found by name and type, not offset, it survives fields moving, nesting, or differing across versions on kernels 5.8 and newer with BTF."
   caption="Figure 58.1 — Relocations name the fields at compile time; the loader patches offsets from the target's BTF" %}

## The portability problem

A kernel struct like `task_struct` is huge and reorganized often. The `pid`
field sits at one byte offset on a 5.15 kernel and possibly a different one on
6.8, because fields were added, removed, or moved ahead of it. Compiled BPF is
just instructions with **baked-in offsets** — "load 4 bytes at task+0x4d0" — so
an object compiled against one kernel's layout silently misreads on another.

Before CO-RE there were two coping strategies, both painful:

- **BCC**: ship the *source* and a compiler (Clang + kernel headers) to every
  target, and compile on the box at runtime. Correct, but heavy — hundreds of
  MB of toolchain and headers, and a compile step on every machine.
- **Per-kernel binaries**: compile a separate object for each kernel version
  you support. A combinatorial maintenance nightmare.

CO-RE, developed by Andrii Nakryiko, gives a third way that's now the default
for serious BPF: compile **once**, against generic type information, and let the
loader fix up the offsets for whatever kernel it lands on — provided that kernel
has **BTF** (any kernel ≥ 5.8 with `CONFIG_DEBUG_INFO_BTF`; BTF itself arrived
in 4.18).

## How CO-RE works

Four building blocks combine, and the book has already used the first one
repeatedly:

- **`vmlinux.h`** — a single header containing *every* kernel type, generated
  from the kernel's own BTF (`bpftool btf dump file /sys/kernel/btf/vmlinux
  format c`). It eliminates any dependency on installed kernel headers: your
  program describes the types it wants from this one generated file.
- **Field relocations** — the heart of it. When your code reads
  `task_struct->pid`, the compiler doesn't just emit "load at offset N." Through
  builtins (`preserve_access_index` and friends), it emits a **BTF relocation**
  that records a *high-level description*: "a field named `pid`, of type
  `pid_t`, in `struct task_struct`." That description, not a number, is what
  ships in the object.
- **The load-time patch** — when the loader (libbpf, or Aya) loads the object on
  a target, it reads that kernel's BTF from `/sys/kernel/btf/vmlinux`, looks up
  where `pid` *actually* lives in *this* kernel's `task_struct`, and rewrites
  the instruction's offset accordingly. Because the match is by **name and
  type**, it works even if `pid` moved, or got wrapped in a nested anonymous
  struct — things invisible in the C source anyway.
- **Kconfig externs and struct flavors** — escape hatches for harder cases
  (config-dependent behaviour, incompatible struct changes) when field
  relocation alone isn't enough.

In C this is explicit: you write `BPF_CORE_READ(task, pid)` instead of a raw
dereference, and nested reads chain — `BPF_CORE_READ(task, nsproxy,
pid_ns_for_children, ns.inum)` — each hop relocated. The relocation isn't only
about offsets; the kinds the compiler can emit include:

- **field offset** — by far the most common (where is `pid`?);
- **field existence** — `bpf_core_field_exists(task->loginuid)` asks *does this
  kernel even have this field?*, so you can branch on it;
- **field size**, and **enum/type** relocations for values and types that
  differ across versions.

Field existence is the one that turns "compiles" into "runs *everywhere*": a
field like `loginuid` may be absent on some kernels, and a CO-RE program checks
before reading rather than assuming.

## How Aya does it

Here's the part that's been invisible: **Aya does CO-RE transparently.** Its BTF
support is "enabled when the target kernel supports it," and when you read a
kernel field through the right types, Aya emits and resolves the relocations for
you — which is exactly why Chapter 15 could load a BTF-aware uprobe and Chapter
52 could touch `task_struct` from a kfunc and have it work. The Rust side needed
new compiler intrinsics (`preserve_access_index`, `preserve_field_info`,
`preserve_type_info`, `preserve_enum_value` for the BPF target), and with those
in place the same relocation machinery libbpf uses applies to Aya objects.

The piece you interact with directly is **`aya-tool`**, which generates the Rust
equivalent of `vmlinux.h` — portable bindings for specific kernel types:

```bash
[host]$ aya-tool generate task_struct > src/vmlinux.rs
```

The docs are explicit that these bindings "are portable across different Linux
kernel versions thanks to CO-RE" — they are *not* simply scraped from your
kernel's headers; reading their fields emits relocations. (`aya-tool` needs
`bpftool` and `bindgen` installed.) The pacing payoff Aya advertises:
**BTF support plus a musl-linked static binary gives a single self-contained
artifact you deploy across distributions and kernel versions** — the operating
goal of this whole part, in one sentence.

## Build, deploy, observe

```bash
cd examples/58-core && ./demo.sh
```

The program reads `pid` and `comm` from the current task portably, and uses a
field-existence check for a field that isn't on every kernel — taking one branch
or the other depending on the target. The demo loads it on the VM (so the
relocations resolve against *that* kernel's BTF) and prints what it read. **In
Grafana**, graph `ebpf_core_reads_total` for the rate of portable reads. The
deeper "observation" is conceptual but real: the *same object* would resolve
different offsets on a different kernel — which is the whole point.

## Cross-check

```bash
[vm]$ ls /sys/kernel/btf/vmlinux                       # the target's BTF must exist
[vm]$ sudo bpftool btf dump file /sys/kernel/btf/vmlinux format c | grep -A3 'struct task_struct'
[vm]$ sudo bpftool prog show                            # the loaded program (offsets already patched)
[vm]$ llvm-objdump -r target/.../core.o | grep -i core  # CO-RE relocation records in the object
```

Seeing CO-RE relocation records in the object — descriptions, not final
offsets — and then a successfully loaded program is the proof the patch happened
at load: the object never contained this kernel's offsets; the loader supplied
them from BTF.

## What you learned

- Kernel struct layouts change, so baked-in offsets break across versions;
  **CO-RE** compiles **once** against generic types and patches offsets at load
  from the target kernel's **BTF** (kernels ≥ 5.8) — replacing BCC's
  compile-on-target and per-kernel binaries.
- The compiler emits **relocations** that name fields by **name and type**
  (offset, **existence** via `bpf_core_field_exists`, size, enum/type), and the
  loader resolves them against `/sys/kernel/btf/vmlinux`; `vmlinux.h` removes the
  kernel-header dependency.
- **Aya does CO-RE transparently**, `aya-tool generate` produces portable Rust
  bindings (needs `bpftool` + `bindgen`), and Aya + musl yields a single static
  binary for a heterogeneous fleet — the foundation everything else in Part 9
  builds on.

Next, Chapter 59 looks at running BPF as a managed service — **lifecycle,
pinning, and zero-downtime upgrades** — now that one artifact can target the
whole fleet.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that `/sys/kernel/btf/vmlinux` exists
(`CONFIG_DEBUG_INFO_BTF`), that `aya-tool generate` produces bindings (needs
`bpftool` + `bindgen`), that the program's field reads resolve and the
existence check branches correctly, and that CO-RE relocation records appear in
the object. For kernels lacking BTF, note BTFHub/external BTF as the fallback.*
