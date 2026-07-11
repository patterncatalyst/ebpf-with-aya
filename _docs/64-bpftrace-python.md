---
title: "bpftrace from Python: one-liners into tools"
order: 64
part: Field guide
description: "bpftrace has been our cross-check all along — the awk-for-the-kernel one-liner we ran to confirm what an Aya program reported. This optional field-guide chapter turns that into a repeatable instrument: bpftrace emits newline-delimited JSON, so a small Python wrapper can run a program, parse the stream by event type, and render a live table or feed a metric — exploration on tap, without writing or compiling Aya."
duration: 35 minutes
---

Part 10 is a field guide to the command-line tools this book has leaned on for
*validation* — the `bpftool`, `bpftrace`, and BCC commands that appeared in
nearly every "Cross-check" section to confirm what our Aya programs reported.
It's optional and tool-focused: the goal is to make you fluent in the
instruments, and to show that each can be **driven from Python** so an ad-hoc
one-liner becomes a repeatable tool. We start with **bpftrace**, the quickest
way to ask the kernel a question.

The code is in `examples/64-bpftrace-python/`. `./demo.sh` runs a Python wrapper
that drives a bpftrace program and renders a live syscall top; the `README.md`
has the details.

{% include excalidraw.html
   file="bpftrace-python"
   alt="Drive bpftrace from Python: one-liners become repeatable, parseable tools. A Python tool using subprocess and json.loads on NDJSON runs bpftrace with -e PROG and -f json. bpftrace compiles the DSL to eBPF via LLVM, attaches, and aggregates in in-kernel maps; the kernel's probes fire and maps aggregate, sending events back. bpftrace writes NDJSON on stdout, which the Python tool dispatches by type — attached_probes, printf, map, hist or lhist, value — into a table, an alert, or an OTel metric. bpftrace is for fast exploration and validation with no compile or deploy; Aya is for production: typed, embeddable, one binary."
   caption="Figure 64.1 — bpftrace compiles and runs the probe; Python parses its JSON stream into a tool" %}

## What bpftrace is

bpftrace is a high-level tracing language — "awk for the kernel." You write a
short program of one or more **probe blocks**, and bpftrace compiles it to eBPF
through the same LLVM BPF backend Aya uses (Chapter 4), attaches it, runs
aggregation in in-kernel maps, and prints results. The shape is always
`probe /predicate/ { action }`:

```
kprobe:vfs_read /comm == "curl"/ { @bytes = hist(arg2); }
tracepoint:raw_syscalls:sys_enter { @[comm] = count(); }
interval:s:1 { print(@); clear(@); }
```

A handful of pieces cover most use:

- **Probes**: `kprobe`/`kretprobe`, `tracepoint`, `uprobe`/`uretprobe`,
  `interval:s:N` (a timer), `begin`/`end` — the same attachment points you met
  across Parts 1–4, named inline.
- **Builtins**: `pid`, `comm`, `args` (tracepoint fields), `arg0..argN` and
  `retval` (probes), `nsecs` — the context, without a struct in sight.
- **Maps and aggregations**: `@name[keys] = count()` / `sum()` / `avg()` /
  `hist()` / `lhist(v, min, max, step)` — in-kernel aggregation, exactly what
  our Aya `HashMap` programs did by hand, here in one line.
- **Output**: `printf(...)` for formatted lines, `print(@)` for a map; bpftrace
  also auto-prints all maps at exit.

This is why it's the perfect cross-check: the runqlat histogram you built over a
whole chapter in Aya is a single `lhist` line in bpftrace, so running both and
comparing is a real test.

bpftrace ships its own collection of ready-made tools, and like the BCC suite
(Chapter 66) they map onto the kernel stack by the layer each observes:

{% include excalidraw.html
   file="bpftrace-tools-map"
   alt="bpftrace tools mapped to the kernel layer each observes. Around the stack — applications, system libraries, system call interface, VFS/sockets/scheduler, file systems/TCP-UDP/virtual memory, block/net devices, device drivers — sit bpftrace tools: vfscount and vfsstat and opensnoop and statsnoop and syncsnoop at VFS; writeback and xfsdist at file systems; mdflush at the volume manager; biosnoop and biolatency and bitesize at the block device; bashreadline at applications; gethostlatency at system libraries; syscount and killsnoop and execsnoop and pidpersec at the syscall interface; cpuwalk and runqlat and runqlen and offcputime at the scheduler; oomkill at virtual memory; tcpconnect and tcpaccept and tcpretrans and tcpdrop at TCP/UDP; and capable as a general security tool."
   caption="Figure 64.2 — bpftrace's own tools, mapped to where each attaches (bpftrace project)" %}

## The key to scripting it: JSON output

Parsing bpftrace's human text is fragile. The `-f json` flag makes it emit
**NDJSON** — newline-delimited JSON, one valid object per line — which is
trivial to consume. Each line is `{"type": ..., "data": ...}`:

- `{"type": "attached_probes", "data": {"probes": 2}}` — startup.
- `{"type": "printf", "data": "small read: 8 byte buffer\n"}` — a `printf`.
- `{"type": "map", "data": {"@": {"curl": 5, "bash": 15}}}` — a `print(@)`.
- histograms arrive as a map of buckets: `{"min": 0, "max": 200, "count": 66}`.
- `{"type": "value", ...}`, `{"type": "stats", ...}`, `{"type": "time", ...}`.

Add `-q` to suppress the chatter and you have a clean machine-readable stream. A
Python tool runs `bpftrace -f json -e '<program>'` as a subprocess, reads stdout
line by line, `json.loads` each, and dispatches on `type`.

## A Python wrapper

The wrapper, `examples/64-bpftrace-python/bpftrace_tool.py`, is small and dependency-free (standard library
only). It launches bpftrace, reads the NDJSON stream, and for each `map` event
renders a top-N table — turning the classic "count syscalls per command" probe
into a live `top`-style view:

```python
proc = subprocess.Popen(
    ["bpftrace", "-q", "-f", "json", "-e", PROGRAM],
    stdout=subprocess.PIPE, text=True)
for line in proc.stdout:
    evt = json.loads(line)
    if evt["type"] == "map":
        render_top(evt["data"]["@"])         # {comm: count} → sorted table
    elif evt["type"] == "attached_probes":
        print(f"attached {evt['data']['probes']} probe(s)")
```

The bpftrace side is the one-liner you'd type by hand; Python adds the parts that
make it a tool — sorting and formatting, a clean refresh each interval, and an
easy hook to do *more* with the numbers than print them: raise an alert past a
threshold, write a CSV, or export an `ebpf_*` metric over OTLP to the same
Grafana you've used all book. The wrapper marks the boundary clearly: bpftrace
gathers, Python decides what the data is *for*.

## The bundled programs

The example ships a set of small, runnable bpftrace programs in `examples/64-bpftrace-python/programs/`,
each mirroring a validation task from earlier in the book. List them with
`bpftrace_tool.py --list`, run any with `--program`, or paste your own with
`-e`. They split into the two output shapes the wrapper handles — **streams**
(`printf`) and **aggregations** (`map`/`hist`):

| Program | What it shows | Output | Mirrors |
|---|---|---|---|
| `syscount.bt` | syscalls per command, per second | table | — |
| `readsize.bt` | `read()` size distribution | histogram | Ch 24 (biopattern) |
| `opensnoop.bt` | every `openat()` (pid, comm, file) | stream | Ch 9 |
| `execsnoop.bt` | every `execve()` (new processes) | stream | Ch 11 |
| `killsnoop.bt` | `kill()` signals (who→whom, signal) | stream | Ch 38 |
| `runqlat.bt` | run-queue (scheduler) latency µs | histogram | Ch 21 |
| `profile.bt` | on-CPU command sampled at 99 Hz | table | Ch 23 |
| `vfsstat.bt` | core VFS ops per second | table | — |
| `tcpconnect.bt` | active TCP connects by command | table | Ch 27 |

Each is a few lines, and running one beside the Aya program it mirrors is a
genuine cross-check: `runqlat.bt`'s histogram should track the one your Aya
runqlat built, `execsnoop.bt`'s stream should match your execsnoop's events. The
point of the set is to *explore* — change a predicate, swap the probe, add a key
to the map, and re-run instantly, with no build step between you and the answer.

## When to reach for this (and when for Aya)

bpftrace driven from Python is the **exploration and validation** instrument:
no crate to scaffold, no compile, no deploy — ask a question, get an answer,
script it if it's worth keeping. It's how you'd confirm, in seconds, that the
counter your Aya program exports matches the kernel's own view. **Aya** is the
**production** instrument: typed programs you embed in a service, ship as one
binary (Chapter 4), attach with lifecycle control (Chapter 59), and run for
months. They're complementary — most real work uses bpftrace to *find* the
probe and the signal, then Aya to *operationalize* it. This chapter, and Part
10, are about being fluent in the first so the second is well-aimed.

## Build, deploy, observe

```bash
cd examples/64-bpftrace-python && ./demo.sh
```

The demo copies the Python wrapper and its bpftrace programs to the lab VM
(where `bpftrace` lives, from Chapter 2), lists the bundled programs, then runs
three of them: the syscall-top table, the `execsnoop` stream, and the `runqlat`
histogram — counts, a live stream, and a distribution, the three shapes you'll
meet. Run any other with `--program`, or an inline one with `-e`. There's no
Grafana panel: these are terminal tools by design (the wrapper notes where an
OTLP export would slot in).

## Cross-check

```bash
[vm]$ sudo bpftrace --info | head                       # build + kernel features bpftrace sees
[vm]$ sudo bpftrace -l 'tracepoint:syscalls:*' | head     # list available probes
[vm]$ sudo bpftrace -q -f json -e 'interval:s:1 { @=count(); print(@); clear(@); }'  # raw NDJSON
```

Seeing the same NDJSON your Python tool consumes, straight from bpftrace, is the
cross-check that the wrapper isn't doing anything magic — it's just parsing a
clean stream the kernel-facing tool already produces.

## What you learned

- **bpftrace** is a high-level tracing DSL — probes, predicates, builtins, and
  in-kernel **maps/aggregations** (`count`, `hist`, `lhist`) — that compiles to
  eBPF, ideal for fast exploration and the cross-checks used throughout the book.
- `-f json` emits **NDJSON** (`{"type","data"}` per line: `attached_probes`,
  `printf`, `map`, histograms, `value`), so a small **Python subprocess wrapper**
  can parse the stream by type and turn a one-liner into a live tool.
- bpftrace is the **exploration/validation** instrument (no build/deploy); **Aya**
  is the **production** one — use bpftrace to find the signal, Aya to ship it.

Next, Chapter 65 drives **bpftool** from Python — inspecting and managing loaded
programs, maps, and links programmatically.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
