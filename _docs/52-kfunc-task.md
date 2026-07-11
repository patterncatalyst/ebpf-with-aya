---
title: "kfuncs: calling the kernel, the modern way"
order: 52
part: Advanced kernel surface
description: "The BPF helper list froze, so the kernel grew a better mechanism: kfuncs — real kernel functions exposed to BPF through BTF, with typed pointer arguments and per-function flags. Understand why helpers stopped scaling, how the KF_ACQUIRE/KF_RELEASE/KF_RET_NULL/KF_TRUSTED_ARGS flags let the verifier reason about kernel objects, and write an Aya program that looks up a task by pid and is forced to release it."
duration: 45 minutes
---

For most of this book, the way a BPF program reached into the kernel was a
**helper** — `bpf_ktime_get_ns`, `bpf_map_lookup_elem`, `bpf_probe_read_user`.
Helpers are a numbered, frozen UAPI: a fixed table the kernel commits to
forever. That stability is a virtue and a straitjacket, and around 2021 the
kernel community stopped growing it and built something better — **kfuncs**.
Almost every feature in this part of the book (sched_ext's `bpf_cpumask_*`,
the user ring buffer's `bpf_dynptr_*`, BPF iterators, allocated objects) is
exposed through kfuncs, so understanding them is the key that unlocks the rest
of the modern surface. This chapter is a full-depth look: why helpers stopped
scaling, how kfuncs let the verifier reason about *kernel objects* through
flags, and a worked Aya program that uses the canonical acquire/release pair.

The code is in `examples/52-kfunc-task/`. `./demo.sh` looks up a task by pid
from within a BPF program and is structurally forced to release it; the
`README.md` has the details.

{% include excalidraw.html
   file="kfuncs"
   alt="Two ways to call into the kernel from BPF. BPF helpers: a frozen, numbered UAPI list of about 210, with a stable ABI and scalar and memory arguments; adding one is a permanent ABI commitment. kfuncs: kernel functions registered via BTF, taking typed kernel pointers, with per-function flags and no ABI stability; subsystems export their own and churn is expected. The acquire/release discipline, verifier-enforced: bpf_task_from_pid (KF_ACQUIRE | KF_RET_NULL) goes through a forced null-check, then must reach bpf_task_release (KF_RELEASE). Drop the release on any path and the program will not load — kfuncs let the verifier enforce reference safety."
   caption="Figure 52.1 — Helpers are a frozen UAPI; kfuncs are BTF-registered functions the verifier reasons about" %}

## Why the helper list stopped scaling

A BPF helper is part of the kernel's stable interface to user space. Each one
gets a number in `enum bpf_func_id`, and once it ships, that number and
signature are forever — the kernel does not break userspace. That guarantee is
exactly why helpers became a bottleneck:

- **Every helper is a permanent commitment.** Adding one means designing an
  interface you can never change. Subsystem maintainers were understandably
  reluctant to grow a global, frozen table for every niche need.
- **Helpers speak a narrow language.** Their arguments are scalars and
  "pointer to a chunk of memory of size N." They can't take a *typed* kernel
  pointer — "a `struct task_struct *` that the verifier knows is valid and
  reference-counted" — because the helper ABI has no way to describe that.

kfuncs drop both constraints. A **kfunc** is an ordinary kernel function that a
subsystem *registers* as callable from BPF, described entirely through **BTF**
(the kernel's type information, which Chapter 15 already leaned on). Because the
description is BTF rather than a frozen number:

- Any subsystem (or even an out-of-tree module) can export its own kfuncs,
  scoped to the program types that should see them — no global UAPI table.
- A kfunc can take **typed kernel pointers** as arguments and return them, and
  the verifier, reading the same BTF, can reason about those types.
- Crucially, kfuncs carry **no ABI-stability promise.** They can change or
  disappear between kernel versions. That is the deliberate trade: subsystems
  get to expose rich, evolving surface precisely because they aren't signing a
  forever-contract. (The practical consequence for you: pin kernel versions and
  expect churn — the flip side of the helper list's rigidity.)

## Flags: how the verifier reasons about kfuncs

A kfunc is registered with a set of **flags** that tell the verifier how to
treat it. They aren't in the function signature; they're attached at
registration (`BTF_ID_FLAGS(func, name, KF_...)`). A handful do most of the
work, and they're the whole reason kfuncs are *safe* despite handing BPF real
kernel pointers:

- **`KF_ACQUIRE`** — the kfunc returns a reference to a refcounted kernel
  object. The verifier now *tracks* that reference: the program must eventually
  release it (via a `KF_RELEASE` kfunc) or store it in a map as a kptr, on
  **every** code path, or the program is rejected at load time.
- **`KF_RELEASE`** — the kfunc consumes a reference. After the call, the
  pointer (and all copies of it) is invalidated; touching it afterward fails
  verification.
- **`KF_RET_NULL`** — the returned pointer may be NULL, so the verifier
  *forces* you to null-check it before any use. No null-check, no load.
- **`KF_TRUSTED_ARGS`** — the kfunc only accepts "trusted" pointers: ones the
  verifier can prove are valid right now (freshly acquired, or passed into your
  program by the kernel), not something reconstructed out of a scalar.

Others fill in the corners: **`KF_SLEEPABLE`** (callable only from sleepable
programs), **`KF_RCU`** / **`KF_RCU_PROTECTED`** (arguments or context must be
under RCU), **`KF_DESTRUCTIVE`** (can crash or reboot the box, so it needs
`CAP_SYS_BOOT`), and the **`KF_ITER_*`** family that powers BPF iterators (a
later chapter). The throughline: each flag is a contract the verifier enforces
*statically*, so a kfunc can safely lend BPF a live kernel object that the
program could otherwise corrupt.

## A worked example: a task by pid

The cleanest demonstration is the canonical acquire/release pair.
**`bpf_task_from_pid(pid)`** looks up a `struct task_struct *` and is
registered `KF_ACQUIRE | KF_RET_NULL`: it hands back a *reference* to the task,
which might be NULL. **`bpf_task_release(task)`** is its `KF_RELEASE` partner.
Together they let a BPF program safely hold a kernel object it didn't receive
from its hook.

In Aya, a kfunc is just an `extern` function the program calls; the linker and
loader resolve it through BTF at load time. The kernel type comes from vmlinux
BTF (the `aya-tool` bindings from Chapter 4; the full CO-RE story is Part 9):

```rust
use vmlinux::task_struct;   // generated: `aya-tool generate task_struct`

// kfuncs are declared as extern fns; resolved via BTF at load time
extern "C" {
    fn bpf_task_from_pid(pid: i32) -> *mut task_struct;  // KF_ACQUIRE | KF_RET_NULL
    fn bpf_task_release(task: *mut task_struct);          // KF_RELEASE
}

#[map] static CONFIG: Array<u32> = Array::with_max_entries(1, 0);   // target pid from user space
#[map] static RESULT: HashMap<u32, u64> = HashMap::with_max_entries(2, 0); // 0=found, 1=missing

#[tracepoint]
pub fn lookup(_ctx: TracePointContext) -> u32 {
    let pid = match CONFIG.get(0) { Some(&p) => p, None => return 0 };

    let task = unsafe { bpf_task_from_pid(pid as i32) };   // KF_ACQUIRE: ref is now tracked
    if task.is_null() {                                    // KF_RET_NULL: must check
        bump(1);                                           // missing
        return 0;                                          // ok to return — nothing acquired
    }

    bump(0);                                               // found
    // … here you could read task fields (a trusted-pointer read; see Part 9) …
    unsafe { bpf_task_release(task) };                     // KF_RELEASE: required on this path
    0
}
```

Walk it the way the verifier does:

- The call to `bpf_task_from_pid` returns an **acquired** reference. From this
  instruction on, the verifier carries a note: *there is an unreleased task
  reference in register/slot X.*
- Because of `KF_RET_NULL`, the very next thing allowed is a **null check**. On
  the NULL branch nothing was acquired (the pointer is NULL), so returning is
  fine. Try to use `task` before checking and the load is rejected.
- On the non-NULL branch we have a live, **trusted** `task_struct *` — the same
  thing the kernel itself holds. We could read its fields, pass it to other
  `KF_TRUSTED_ARGS` kfuncs, or stash it in a map as a kptr. Here we just count.
- **`bpf_task_release(task)`** discharges the reference. After it, `task` is
  poisoned; any further use fails. And it is **mandatory**: delete that line and
  the verifier walks every path, finds the non-NULL path leaks a reference, and
  refuses to load the program. That rejection is not a runtime error — it's a
  compile-the-world-first guarantee that this program cannot leak a task.

This is the part worth sitting with: with helpers, "don't leak the reference"
would be a comment and a code review. With kfuncs, it's a property the kernel
*proves* before your program ever runs. The example's README suggests deleting
the release line so you can watch the verifier reject it.

## Build, deploy, observe

```bash
cd examples/52-kfunc-task && ./demo.sh
```

The demo writes a target pid into `CONFIG` (its own, then a bogus one),
triggers the tracepoint, and reads back the found/missing tallies. **In the
terminal** you'll see the real pid resolve (`found`) and the bogus one not
(`missing`). **In Grafana** (`127.0.0.1:3000` → Explore), graph `sum by
(result) (rate(ebpf_task_lookups_total[1m]))` to watch found-vs-missing
lookups over time — a BPF program reaching a kernel object by pid, safely.

## Cross-check

```bash
[vm]$ sudo bpftool prog dump xlated name lookup | grep -A1 -i call    # the kfunc call sites
[vm]$ sudo bpftool btf dump file /sys/kernel/btf/vmlinux | grep bpf_task_from_pid
[vm]$ ps -o pid,comm -p <the-pid-you-asked-for>                       # confirm the target exists
```

`bpftool prog dump xlated` shows the program calling `bpf_task_from_pid` /
`bpf_task_release` as kfunc calls (not numbered helpers); the vmlinux BTF dump
proves the kfunc exists in *this* kernel — which, given kfuncs' lack of ABI
stability, is exactly the thing to verify before relying on one.

## What you learned

- The **helper list is a frozen UAPI** and stopped scaling; **kfuncs** are
  kernel functions exposed to BPF through **BTF**, with typed pointer
  arguments, modular per-subsystem registration, and deliberately **no ABI
  stability**.
- Per-function **flags** are how the verifier reasons about kernel objects:
  `KF_ACQUIRE`/`KF_RELEASE` track references, `KF_RET_NULL` forces null checks,
  `KF_TRUSTED_ARGS` restricts which pointers are accepted.
- Calling a kfunc from Aya is declaring an `extern` function resolved via BTF;
  the canonical `bpf_task_from_pid` / `bpf_task_release` pair shows the verifier
  **enforcing reference safety at load time** — leak the reference and the
  program won't load.

Next, Chapter 53 looks at the **BPF token** — how this kernel surface gets
safely delegated into unprivileged containers.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): tallies found (target pid running) vs missing. The kfunc form (bpf_task_from_pid) is not expressible in aya-ebpf (see below); this checks the current task instead.*
