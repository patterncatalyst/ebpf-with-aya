---
title: "BTF-assisted uprobe"
order: 15
part: User-space & language probing
description: Read a struct argument from a uprobe by following a pointer into the target's memory, and understand how BTF recovers a struct's layout so you can probe binaries you didn't write without hardcoding offsets.
duration: 25 minutes
---

Chapter 14 read a scalar argument — a `u64` in a register. Real
functions take **pointers to structs**, and reading fields out of them
brings back a problem you met in the kprobe chapter: you need to know
the struct's layout, and hardcoding offsets is fragile. This chapter
reads a struct argument the robust way, and explains how **BTF** —
the same type-information format that powers kernel CO-RE — lets you do
it even for binaries whose source you don't have.

The code is in `examples/15-btf-uprobe/`.

{% include excalidraw.html
   file="struct-btf"
   alt="Reading a struct argument: the uprobe takes the pointer argument and copies the struct with bpf_probe_read_user into a repr(C) mirror whose field layout must match the target's — a layout BTF can recover."
   caption="Figure 15.1 — struct-argument read and the BTF layout contract" %}

## Reading a struct argument

The target, `process_order(const Order *order)`, is passed a pointer to
an `Order { id, amount_cents, status }`. A uprobe gets the pointer from
`arg(0)`, but the pointer is just an address in the *target process's*
memory — you can't dereference it directly (the verifier forbids it,
same as kernel pointers). You copy the struct out:

```rust
let order_ptr: *const Order = ctx.arg(0).ok_or(0)?;
let order: Order = unsafe { bpf_probe_read_user(order_ptr)? };
```

`bpf_probe_read_user::<Order>(ptr)` copies `size_of::<Order>()` bytes
from the target's address space into a local `Order`. The whole struct
comes across in one read; then you have a normal value to pull fields
from. That's the mechanic — and it's solid and works today.

## The catch: layout has to match

`bpf_probe_read_user::<Order>` copies *bytes* and reinterprets them as
an `Order`. If your program's idea of `Order`'s layout — field order,
sizes, padding — doesn't match the target's, you read garbage. In this
example we sidestep the problem: the target app, the eBPF program, and
user space all use the **same** `Order` definition from
`btf-uprobe-common`, with `#[repr(C)]` so the layout is fixed and
shared. Correct by construction.

But that only works because we *wrote* the target. The interesting
real-world case is probing a binary you **didn't** write — a database, a
language runtime, a service whose source you don't have. You can't
share its struct definition. So how do you know the layout?

## BTF: the layout, recovered

**BTF** (BPF Type Format) is compact type information describing
structs, their fields, and their offsets. You met it as the kernel's
`/sys/kernel/btf/vmlinux` in Chapter 2, and as what makes fentry's typed
arguments and CO-RE's relocations work (Chapters 5, 8). The same idea
applies to user-space binaries: if a binary carries BTF, it tells you
exactly how its structs are laid out.

Most binaries ship with DWARF debug info rather than BTF, but you can
convert: `pahole -J` (from Fedora's `dwarves` package) reads a binary's
DWARF and writes a `.BTF` section into it, which `bpftool` can then
dump. On the target VM:

```bash
[vm]$ sudo dnf install -y dwarves
[vm]$ pahole -J /home/fedora/target-app
[vm]$ bpftool btf dump file /home/fedora/target-app | grep -iA4 order
```

That dump shows `Order` with its members and their byte offsets. That
is the ground truth you'd use to write a correct `#[repr(C)]` mirror for
a target you don't control — and, with the emerging user-space CO-RE
support, the relocation source that lets the loader fix up offsets at
attach time so your probe keeps working even if the target's struct
changes between versions.

> **Scope note.** Kernel CO-RE (relocating against
> `vmlinux` BTF) is mature and turnkey in Aya. *User-space* CO-RE —
> relocating against a target binary's BTF — is newer and less
> push-button. The reliably-works-today technique is the one in this
> chapter: a `#[repr(C)]` mirror (shared when you control the source,
> or **generated from the target's BTF** when you don't), read with
> `bpf_probe_read_user`. BTF is what makes that mirror *correct* and
> lets you detect when a target's layout has drifted out from under
> you. The full relocation story is the CO-RE deep-dive in Chapter 56.

## Build, deploy, observe

```bash
cd examples/15-btf-uprobe && ./demo.sh
```

The demo ships `target-app` to the VM (built with debug info so it
carries DWARF), starts it submitting an order every half second, and
attaches the uprobe. You'll see the struct fields, decoded:

```text
PID      ID       AMOUNT       STATUS
12345    1000     $0.00        received
12345    1001     $9.99        paid
12345    1002     $19.98       shipped
```

The `status` enum, the `amount_cents` integer formatted as currency —
all read live out of the struct the target passed, by a probe in the
kernel. `ebpf_events_total{program="btf-uprobe",status=…}` breaks the
orders down by status in Grafana.

## Cross-check

```bash
[vm]$ bpftool btf dump file /home/fedora/target-app | grep -iA4 order
```

If the offsets in the BTF dump match how your `#[repr(C)] Order` lays
out (`id` at 0, `amount_cents` at 8, `status` at 16), your reads are
reading the right bytes. When they *don't* match — different compiler,
different version, `#[repr(Rust)]` reordering — that mismatch is exactly
what BTF exists to catch.

## What you learned

- Read a struct argument by taking the pointer from `arg(0)` and
  copying it with `bpf_probe_read_user::<T>` — never dereference target
  pointers directly.
- The copy is only correct if your `#[repr(C)]` layout matches the
  target's; sharing the definition is the easy case.
- **BTF** recovers a binary's struct layout (`pahole -J` +
  `bpftool btf dump`), which is how you build a correct mirror — and
  eventually relocate offsets — for targets you don't control.

Next, the part turns to language runtimes and userspace targets in
earnest (Java/Python bootstrap, then `sslsniff`, `funclatency`).

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm `bpf_probe_read_user::<T>` reading a whole struct, the `arg(0)`
pointer, attachability under release+LTO, and that `debug = true` leaves
usable DWARF for `pahole -J`. The first build and run are the test.*
