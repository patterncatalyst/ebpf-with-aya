---
title: "Dynptrs and arenas: flexible memory for BPF"
order: 56
part: Advanced kernel surface
description: "Classic BPF memory is rigid: fixed-size maps, fixed-size stack, bounds the verifier can prove at compile time. Two modern primitives loosen that. A dynptr is a verifier-tracked handle to a variable-length region — the thing the user ring buffer handed you. A BPF arena is a sparse memory region shared zero-copy with user space, where BPF builds real pointer-based data structures. Learn both, and when each earns its place."
duration: 45 minutes
---

The verifier's love of fixed sizes has shaped every program in this book. Maps
have a fixed value size; the stack is 512 bytes; when we captured a SQL string
in Chapter 47 we read it into a *fixed* 128-byte buffer and truncated, because
"variable length" and "provable bounds" are in tension. Two recent additions
resolve that tension from opposite ends. **Dynptrs** (kernel 5.19) give you a
safe handle to a region whose length is only known at runtime — you already met
one as the user-ring-buffer sample in Chapter 50. **Arenas** (kernel 6.9) give
you a large region of memory, shared zero-copy with user space, in which BPF
can build *real data structures with real pointers*. Together they're how
modern BPF escapes the fixed-size box.

The code is in `examples/56-dynptr-arena/`. `./demo.sh` writes variable-length
records through a dynptr and points at an arena built structure; the
`README.md` has the details.

{% include excalidraw.html
   file="dynptr"
   alt="One safe handle for variable-length memory, whatever backs it. On the left, backing stores: local memory, a ringbuf reservation, an skb or xdp packet, and a user-ringbuf sample. They are wrapped by a bpf_dynptr in the middle — a handle, not a raw pointer. On the right, bpf_dynptr_data(offset, len) returns a bounds-checked slice and the verifier enforces no out-of-bounds access. skb and xdp use bpf_dynptr_slice because data may be non-contiguous, and the slice is invalidated when the dynptr is."
   caption="Figure 56.1 — A dynptr is a verifier-tracked handle; data access returns a bounds-checked slice" %}

## Dynptrs: safe variable-length access

The problem a dynptr solves is precise. The verifier must prove every memory
access is in bounds, and the easy way to do that is to make sizes constant.
But real data — a packet payload, a log line, a user-submitted message — has a
length you only learn at runtime. Without dynptrs you cope by copying into a
fixed buffer and truncating, which is wasteful and lossy.

A **dynptr** is a *handle* to a region, not a raw pointer. It carries the
region's bounds with it, and the verifier tracks those bounds across your
program. You don't dereference it directly; you ask it for access:

- **`bpf_dynptr_data(&dynptr, offset, len)`** returns a pointer to `len` bytes
  at `offset` — and `len` must be a compile-time constant, so the verifier can
  treat the result as a normal buffer of known size with no out-of-bounds
  access possible. This is the fast path: a direct view into the region.
- **`bpf_dynptr_read`/`bpf_dynptr_write`** copy bytes in or out when you don't
  want a direct slice.
- **`bpf_dynptr_slice`/`bpf_dynptr_slice_rdwr`** are for **skb/xdp** dynptrs,
  where the data may not be contiguous: they either hand back a direct pointer
  or copy into a buffer *you* supply, and may return NULL — so the verifier
  forces a null check (the `KF_RET_NULL` discipline from Chapter 52).

The unifying idea is that a dynptr abstracts over *what backs the memory*. The
same handle type wraps local memory (`bpf_dynptr_from_mem`), a ring-buffer
reservation (`bpf_ringbuf_reserve_dynptr`), packet data
(`bpf_dynptr_from_skb`/`_xdp`), and the user-ring-buffer sample you drained in
Chapter 50. Write your parsing once against a dynptr and it works regardless of
source. One caution worth internalizing: a slice is **invalidated when its
dynptr is** (e.g., once you submit a ring-buffer reservation), so you don't
hold slices across the operation that ends the region.

The worked example uses the ring-buffer dynptr to emit **variable-length
records** — short or long depending on the event — instead of padding every
record to a fixed maximum:

```c
SEC("tracepoint/syscalls/sys_enter_getpid")
int emit(void *ctx) {
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 len = sizeof(struct hdr) + payload_len_for(pid); /* runtime length */
    struct bpf_dynptr d;
    if (bpf_ringbuf_reserve_dynptr(&rb, len, 0, &d) == 0) {
        struct hdr *h = bpf_dynptr_data(&d, 0, sizeof(*h)); /* checked slice */
        if (h) { h->pid = pid; h->len = len; }
        /* write only the bytes this record needs */
    }
    bpf_ringbuf_submit_dynptr(&d, 0);   /* slice now invalid */
    return 0;
}
```

Each record occupies exactly what it needs; user space reads the length from
the header and consumes that many bytes. No fixed maximum, no truncation.

## Arenas: a shared heap for real data structures

Dynptrs make *access* flexible; **arenas** make *structure* flexible. Until
recently, building a hash table or a tree inside BPF meant abusing a giant
array map and using integer indices where you wanted pointers — ugly, slow, and
nothing user space could read without a syscall per element. A **BPF arena**
(`BPF_MAP_TYPE_ARENA`) is a sparse memory region — up to 4 GB — that both the
BPF program and user space can address with **real pointers**.

{% include excalidraw.html
   file="bpf-arena"
   alt="BPF arena: a shared heap where BPF builds real pointer data structures. On the left a BPF program calls bpf_arena_alloc_pages, uses __arena pointers, and builds a list, tree, or hash. In the center is the BPF arena, a sparse shared region up to 4 GB. On the right, user space mmaps the same region and reads the structures directly with no per-access syscall; the mmap link is zero-copy and bidirectional. Before arena, developers faked pointers with array-map indices; now lists, trees, and graphs use normal pointers, shared zero-copy."
   caption="Figure 56.2 — An arena is a shared region where BPF builds pointer-based structures user space mmaps" %}

How it works:

- The BPF program allocates from the arena with **`bpf_arena_alloc_pages`**
  (and there are small allocators built on top), and refers to arena memory
  through pointers tagged `__arena` (a separate address space the compiler
  understands with `-D__BPF_FEATURE_ADDR_SPACE_CAST`). Within the arena you use
  **ordinary C pointer operations** — `node->next = new_node` — to build linked
  lists, trees, hash tables with chaining, graphs.
- **User space `mmap`s the same arena** and sees the identical bytes at its own
  address. Access is **zero-copy and bidirectional**: no `bpf_map_lookup_elem`
  syscall per element, no copying. A user program can walk a list BPF built, or
  seed data BPF will read.

The use cases this unlocks are real: an in-kernel **key-value accelerator**
(XDP looks up a key in the arena and answers without ever going to user space),
custom in-kernel data structures, or a BPF "heap" of up to 4 GB
(`BPF_F_NO_USER_CONV` when you don't even need to share it). The trade-off
versus maps is exactly about pointers and access patterns: if you find yourself
encoding indices to fake pointers in a map, you want an arena; if you need a
large structure both sides touch, the zero-copy sharing is hard to beat.

## Where Aya fits

Both are frontier for Aya. Dynptr support in aya-ebpf is **emerging** — the
ring-buffer and local variants are the most likely to be usable — while arena,
which leans on the compiler's address-space-cast feature and very new verifier
support, is **nascent**. So `examples/56-dynptr-arena/` keeps the canonical C in
`reference/` (a dynptr ring-buffer producer and an arena linked list), provides
an Aya rendering of the dynptr side, and a **real Aya loader** that reads the
variable-length records. The arena piece is built and inspected with
`bpftool`, the production path while Aya catches up.

## Build, deploy, observe

```bash
cd examples/56-dynptr-arena && ./demo.sh
```

The demo loads the dynptr producer, generates events, and reads the
variable-length records back — printing each record's actual length so you can
see them differ. **In Grafana** (`127.0.0.1:3000` → Explore), graph
`rate(ebpf_dynptr_records_total[1m])` for the record rate. It then compiles the
arena example with `bpftool`, and (where supported) shows user space reading the
BPF-built structure straight out of the mmap'd arena — no syscalls in the read
path.

## Cross-check

```bash
[vm]$ sudo bpftool map show | grep -E 'ringbuf|arena'   # the backing maps
[vm]$ sudo bpftool map dump name arena_map | head        # arena bytes, if built
```

Seeing records of *different* lengths arrive intact is the dynptr cross-check;
seeing the arena map's bytes change as the BPF program builds its structure —
readable without a per-element syscall — is the arena one.

## What you learned

- A **dynptr** is a verifier-tracked **handle** to a variable-length region;
  `bpf_dynptr_data(off, len)` returns a bounds-checked slice (constant `len`),
  `slice`/`slice_rdwr` cover non-contiguous skb/xdp, and the same handle
  abstracts over local/ringbuf/packet/user-ringbuf backing — the thing Chapter
  50 handed your callback.
- A **BPF arena** is a sparse, up-to-4 GB region shared **zero-copy** with user
  space, where BPF builds real pointer-based data structures
  (`bpf_arena_alloc_pages`, `__arena` pointers) instead of faking pointers with
  map indices.
- Reach for a **dynptr** when length is dynamic, an **arena** when you need rich
  structures or large shared memory; both are emerging in Aya, with C canonical
  for now.

Next, Chapter 57 closes the advanced part with **BPF iterators** — walking
kernel data structures and emitting their contents like a synthetic file.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: the dynptr ring-buffer API
(`bpf_ringbuf_reserve_dynptr`/`bpf_dynptr_data`/`bpf_ringbuf_submit_dynptr`,
kernel ≥ 5.19) and the Aya rendering of it; that variable-length records arrive
intact; and for arena (kernel ≥ 6.9) that it compiles with
`-D__BPF_FEATURE_ADDR_SPACE_CAST`, loads via `bpftool`, and user-space mmap
reads the BPF-built structure. Treat the aya-ebpf dynptr/arena support as
emerging.*
