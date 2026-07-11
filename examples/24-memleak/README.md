# Example 24 — memleak (outstanding-allocation tracker)

Find allocations that are never freed, and the **call stack** that made
them — the eBPF answer to "what's leaking?"

## What this shows

- Pairing **`malloc`/`calloc`** (uprobe + uretprobe) with **`free`**
  (uprobe) on libc: record `ALLOCS[ptr] = {size, stack}` on allocation,
  delete it on free. Whatever remains is outstanding.
- Reusing **stack walking** from Ch 23: `bpf_get_stackid` captures the
  *user* stack at the allocation site, so each leak names its origin.
- **Filtering to one pid** in-kernel (libc malloc fires constantly) via
  a `TARGET_PID` config map.
- Grouping outstanding allocations by stack in user space → "N bytes in
  M allocations from this stack".

## Build (target needs clang on the VM)

The demo compiles the bundled `leaker.c` on the VM with
`-fno-omit-frame-pointer` so the user stack is walkable.

```bash
./demo.sh build     # build memleak on host
./demo.sh           # compile+run leaker on VM, watch 15s, report leaks
SECS=30 ./demo.sh
```

You should see ~4 KiB accumulating from the `leak_here` call site and
nothing from the balanced `use_and_free` site. `memleak_outstanding_bytes`
in Grafana climbs over time for a real leak.

## Cross-check (on the VM)

```bash
[vm]$ sudo memleak-bpfcc -p $(pgrep leaker) 5
```

## ⚠ Verification status

**Unverified.** Risks: uprobe+uretprobe on `malloc`/`calloc` + uprobe on
`free` in libc (symbol names on Fedora's glibc); `ctx.arg`/`ctx.ret` and
`get_stackid` in aya 0.14.x; user-stack capture needing frame pointers
(hence `-fno-omit-frame-pointer` on the target — glibc itself may still
be FP-omitted, which can truncate stacks; note for verification);
`Array::set` pid filter; `u64_gauge` in opentelemetry 0.27. User-frame
symbolization is hex (wire in blazesym). `realloc`/`posix_memalign` are
not traced — a documented gap. Record results in
`_plans/reconciliation-plan.md`.
