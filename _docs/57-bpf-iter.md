---
title: "BPF iterators: walking the kernel"
order: 57
part: Advanced kernel surface
description: "Closing the advanced part with one more inversion. Instead of reacting to an event, a BPF iterator is the body of an in-kernel loop: the kernel walks a set — every task, every socket, every map entry — and calls your program once per element, which emits through a seq_file you can cat. Learn the iterator program type, the open-coded iterators that put bounded loops anywhere, and why pushing iteration into the kernel beats dragging state to user space."
duration: 40 minutes
---

This part has been a tour of inversions: kfuncs let BPF call the kernel,
struct_ops let BPF implement the kernel, timers let BPF schedule the kernel. The
last one is about *reading* the kernel. Every program so far has waited for the
kernel to reach an event — a packet, a syscall, a scheduler tick. A **BPF
iterator** flips that: the kernel walks an entire *set* of objects — all tasks,
all sockets, all entries of a map — and calls your program **once per element**.
You're not reacting to one event; you're the body of an in-kernel loop over
kernel state. It's a fitting close to the advanced surface, because it changes
where the work happens: instead of dragging kernel state to user space and
sifting it there, you push the sifting *into the kernel where the data lives.*

The code is in `examples/57-bpf-iter/`. `./demo.sh` builds a task iterator,
pins it, and `cat`s a process table your BPF program produced; the `README.md`
has the details.

{% include excalidraw.html
   file="bpf-iter"
   alt="Iterators flip the model: the kernel calls your program for every element of a set. On the left, a kernel set — all tasks, all TCP sockets, all map elements. The kernel drives one call per element into the BPF iterator program in the middle, which runs once per element and calls BPF_SEQ_PRINTF. On the right, the seq_file output is read by cat /sys/fs/bpf/task_iter or by read() on the link fd. Open-coded iterators (bpf_for or KF_ITER kfuncs) put bounded loops inside any BPF program."
   caption="Figure 57.1 — The kernel drives the loop; your program is the per-element body, emitting to a seq_file" %}

## Why iterate in the kernel

Consider a seemingly simple task: "list every process whose name is `nginx`,
with its open file count." The traditional way is to walk `/proc` from user
space — open thousands of files, parse text, race against processes coming and
going. It's slow, fragile, and inconsistent (the set changes while you read
it). The data lives in the kernel; you're hauling all of it out just to throw
most of it away.

A BPF iterator reverses the flow. You write a program that the kernel calls for
each `task_struct`, with **direct access to its fields**; you filter and format
in C, in-kernel, and only the result crosses to user space. No `/proc` parsing,
far less data copied, and a more consistent snapshot. The same applies to
sockets (dump every TCP connection without `ss` parsing `/proc/net/tcp`), to
cgroups, to BTF symbols, and — importantly — to **map elements**, which lets you
walk and aggregate a map's contents entirely in the kernel.

The kernel doc names two related things under "BPF iterators," and it's worth
keeping them straight:

- The **iterator program type** — a standalone program the kernel calls once
  per element of a chosen set. This is the headline feature and our focus.
- **Open-coded iterators** — BPF-side APIs (`bpf_for`, `bpf_repeat`, the
  `bpf_iter_*` `KF_ITER_*` kfuncs from Chapter 52) that let *any* program type
  run a bounded loop. Same idea, different scope: loops inside a program rather
  than a dedicated iterator program.

## The iterator program type

You mark a program `SEC("iter/<target>")` — `iter/task`, `iter/task_file`,
`iter/tcp`, `iter/bpf_map_elem`, and so on. The kernel calls it once per
element, handing it a context with two parts: a **`meta`** (carrying the
`seq_file` to write to, plus a `seq_num` counter and session id) and a **pointer
to the current element** (`task`, `file`, the map key/value…). A task iterator:

```c
unsigned long count = 0;

SEC("iter/task")
int dump_task(struct bpf_iter__task *ctx)
{
    struct seq_file *seq = ctx->meta->seq;
    struct task_struct *task = ctx->task;

    if (task == NULL) {                          /* end of iteration */
        BPF_SEQ_PRINTF(seq, "total: %lu tasks\n", count);
        return 0;
    }
    if (ctx->meta->seq_num == 0)                 /* first call: header */
        BPF_SEQ_PRINTF(seq, "%-8s %-8s %s\n", "TGID", "PID", "COMM");

    BPF_SEQ_PRINTF(seq, "%-8d %-8d %s\n", task->tgid, task->pid, task->comm);
    count++;
    return 0;
}
```

Reading it the way the kernel drives it:

- The kernel invokes `dump_task` **once for every `task_struct`** in the system,
  in turn. Inside, `task` is a real, valid kernel pointer — you read
  `task->tgid`, `task->pid`, `task->comm` directly (BTF/CO-RE makes the field
  offsets portable), and you could filter on anything (`if
  (!streq(task->comm, "nginx")) return 0;`).
- **`ctx->meta->seq_num == 0`** marks the first call, so you print a header
  exactly once; a **NULL `task`** marks the end, so you print a summary. Those
  two sentinels bookend the stream.
- **`BPF_SEQ_PRINTF`** writes into the kernel's **seq_file** — the same
  machinery `/proc` files use. That's what makes the output readable as a file.

You attach it via a link and **pin it to bpffs**, then read it like any file:

```bash
[vm]$ sudo bpftool iter pin task_iter.o /sys/fs/bpf/task_iter
[vm]$ sudo cat /sys/fs/bpf/task_iter        # a process table your BPF built
TGID     PID      COMM
1        1        systemd
...
total: 312 tasks
```

Every `cat` re-runs the iteration fresh. User space can also `read()` the link
fd directly instead of pinning. A **map-element** iterator (`iter/bpf_map_elem`)
takes the target map as a parameter (`bpftool iter pin obj path map MAP`), so
your program is called once per entry — in-kernel aggregation or GC over a map's
contents.

## Open-coded iterators, briefly

The iterator *program type* walks kernel-defined sets. Sometimes you just want a
**bounded loop inside an ordinary program** — and the verifier has historically
been hostile to loops. Open-coded iterators are the sanctioned way: the
`bpf_iter_*` kfuncs (`bpf_iter_num_new`/`_next`/`_destroy`, flagged
`KF_ITER_NEW`/`NEXT`/`DESTROY` as Chapter 52 noted) and the `bpf_for(i, 0, n)`
and `bpf_repeat(n)` macros built on them. The verifier understands these as
terminating, so you can iterate a range, or walk tasks/cgroups
(`bpf_iter_task`, `bpf_iter_css`) from inside a kprobe or tracepoint — no
dedicated iterator program required. Same concept (iteration as a first-class,
verifiable construct), broader reach.

## Where Aya fits

The iterator program type is **emerging** in aya — the same frontier theme of
this part — so the example keeps the canonical iterator in
`examples/57-bpf-iter/reference/task_iter.bpf.c` and pins it with **`bpftool
iter pin`**, the production path that needs no Aya at all, with an Aya rendering
to read. The conceptual model is what matters and it's identical across loaders:
mark `iter/<target>`, write the per-element body, emit through the seq_file,
pin, read.

## Build, deploy, observe

```bash
cd examples/57-bpf-iter && ./demo.sh
```

The demo compiles the task iterator, pins it, and `cat`s it so you see a process
table assembled entirely in the kernel — header, one row per task with fields
read straight from each `task_struct`, and a total. There's no Grafana panel
here, and that's the honest shape of an iterator: **its output *is* the result**
— a stream you read, not a metric time series. (If you wanted a metric, a reader
could `read()` the iterator and count matching elements; the chapter leaves the
output as the dump it naturally is.)

## Cross-check

```bash
[vm]$ sudo cat /sys/fs/bpf/task_iter | wc -l    # rows produced
[vm]$ ps -e | wc -l                              # the system's task count, for comparison
[vm]$ sudo bpftool iter help                     # the iterator targets bpftool can pin
```

The row count from your iterator tracking `ps` is the cross-check that you
walked the real task set; comparing fields for a known pid against `ps`
confirms you read them correctly.

## What you learned

- A **BPF iterator** is the body of an in-kernel loop: the kernel walks a set
  (tasks, sockets, **map elements**, …) and calls your program **once per
  element**, which reads fields directly and emits through a **seq_file** you
  `cat` — pushing filtering to where the data lives instead of parsing `/proc`.
- The program is `SEC("iter/<target>")`; `meta->seq_num == 0` marks the header
  call and a NULL element marks the end; you pin with `bpftool iter pin` and
  read the result as a file.
- **Open-coded iterators** (`bpf_for`, `bpf_iter_*` kfuncs) bring the same
  bounded-iteration idea into any program type. Iterator program support is
  emerging in Aya; the C path via `bpftool` is canonical today.

That closes the advanced kernel surface. Part 9 — **Operating eBPF** — steps up
a level to running this in production, beginning with the **CO-RE** deep-dive
the earlier chapters kept pointing forward to.

---

*Verification status: <span class="status status--verified">verified</span>
— Fedora 44, kernel 7.1.3 (clang 22, bpftool v7.6.0). The task iterator compiles
against this kernel's `vmlinux.h`, `bpftool iter pin` pins it, and `cat`ing the
pin produces the process table (469 tasks) with `tgid`/`pid`/`comm` — assembled
entirely in the kernel. The aya-ebpf iterator rendering stays emerging — the C
path is canonical.*
