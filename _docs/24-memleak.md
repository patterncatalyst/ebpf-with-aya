---
title: "memleak"
order: 24
part: Performance & resources
description: "Find allocations that are never freed by pairing malloc/calloc with free on libc, recording each outstanding allocation's call stack with bpf_get_stackid so the leak points back to the code that made it."
duration: 30 minutes
---

A memory leak isn't a crash — it's bytes that pile up because something
allocated them and never freed them. `memleak` catches them in the act:
watch every `malloc`/`calloc` and every `free`, and whatever's allocated
but not freed by the end is a candidate leak. The trick that makes it
*actionable* is recording the **call stack** at each allocation, so the
leak names the code that caused it. This chapter reuses the stack-walking
from `profile` for exactly that.

The code is in `examples/24-memleak/`.

{% include excalidraw.html
   file="memleak-tracking"
   alt="Leak tracking: malloc records each allocation's pointer, size, and call stack; free removes it; whatever remains outstanding is a candidate leak, grouped by the stack that allocated it."
   caption="Figure 24.1 — tracking outstanding allocations by call site" %}

## The idea: a live allocation table

The bookkeeping is simple:

- **`malloc(size)`** — on entry, stash the requested size; on **return**,
  the pointer is the result, so record `ALLOCS[ptr] = { size, stack }`.
- **`calloc(n, sz)`** — same, with `size = n * sz`.
- **`free(ptr)`** — on entry, delete `ALLOCS[ptr]`.

Run it for a while, and `ALLOCS` holds exactly the allocations that are
still outstanding. Group them by allocation stack and you get "this code
path has 4 KiB outstanding across N allocations" — the leak, with a
return address trail.

## malloc needs entry **and** return

`malloc`'s argument (the size) is available at **entry**, but its result
(the pointer) only at **return** — the same split you saw with `SSL_read`
in Chapter 17. So `malloc` needs both a uprobe and a uretprobe, bridged
by a per-thread stash:

```rust
#[uprobe]    fn malloc_enter(ctx) { SIZES.insert(&pid_tgid(), &ctx.arg(0)?, 0); }
#[uretprobe] fn malloc_exit(ctx)  {
    let size = SIZES.get(&pid_tgid())?;       // from entry
    let ptr  = ctx.ret()?;                     // the allocation
    let stack = STACKS.get_stackid(ctx, BPF_F_USER_STACK)?;  // who allocated it
    ALLOCS.insert(&ptr, &AllocInfo { size, stackid: stack, pid }, 0);
}
```

`free` is just an entry probe that removes the pointer. (libc tolerates
uretprobes fine — unlike Go in Chapter 19.)

## Recording the allocation site

The line that makes this tool useful is `STACKS.get_stackid(ctx,
BPF_F_USER_STACK)` — the same stack-capture primitive from `profile`,
here grabbing the **user** stack at the moment of allocation and storing
its id alongside the size. Identical allocation sites share a stack id,
so grouping outstanding allocations by `stackid` in user space collapses
them into per-call-site leak totals.

This is why `profile` came first: stack walking is a reusable building
block, not a one-off.

## Filtering to one process

`malloc` is one of the busiest functions on the system — uprobing it
system-wide would fire constantly. So `memleak` scopes to a single pid
with a `TARGET_PID` config map, set before attaching, and the probes bail
out early for any other process:

```rust
fn skip(pid: u32) -> bool {
    let target = TARGET_PID.get(0).copied().unwrap_or(0);
    target != 0 && pid != target
}
```

That early return keeps the overhead proportional to *one* process's
allocation rate, not the whole machine's.

## A note on frame pointers

Walking a user stack needs a way to unwind it. The cheap, reliable way
is **frame pointers**, which is why the bundled `leaker.c` is built with
`-fno-omit-frame-pointer`. Code compiled *without* frame pointers (much
of a typical distro, including parts of glibc) can give truncated stacks
from `bpf_get_stackid` — a real limitation worth knowing. Production
setups either compile hot paths with frame pointers or use DWARF-based
unwinding in the symbolizer.

## Build, deploy, observe

The demo compiles `leaker.c` on the VM (it leaks 4 KiB every fourth
iteration from one call site, while a second site allocates and frees
cleanly), starts it, and watches its pid:

```bash
cd examples/24-memleak && ./demo.sh
```

After the window, you'll see outstanding bytes accumulating from the
`leak_here` stack and nothing from `use_and_free` —
`memleak_outstanding_bytes` rises steadily in Grafana, the signature of
a real leak versus steady-state churn. User frames print as hex; wire in
`blazesym` to turn them into `leak_here`, `main`, and friends.

## Cross-check

```bash
[vm]$ sudo memleak-bpfcc -p $(pgrep leaker) 5
```

The BCC tool reports the same outstanding allocations and stacks.

## What you learned

- Pair `malloc`/`calloc` (entry+return) with `free` (entry) to maintain
  a **live allocation table**; the remainder is the leak.
- Capture the **allocation stack** with `bpf_get_stackid` so leaks point
  back to their source — the `profile` primitive, reused.
- **Scope to a pid** in-kernel for a hot function like `malloc`.
- User-stack unwinding depends on **frame pointers**.

Next: **`biopattern`**, classifying block I/O as sequential vs. random.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Highest-risk: uprobe+uretprobe on glibc `malloc`/`calloc` + uprobe on
`free`; user-stack capture depending on frame pointers (glibc itself may
truncate); `Array::set` pid filter and `u64_gauge` in
opentelemetry 0.27. `realloc`/`posix_memalign` are untraced gaps. The
first build and run are the test.*
