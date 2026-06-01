---
title: "The BCC tools: the ready-made suite"
order: 66
part: Field guide
description: "BCC ships the ~100 ready-made tracers whose names recur in every cross-check — execsnoop, biolatency, runqlat, tcpconnect, profile. This closing field-guide chapter tours them, explains the two generations (classic Python+C that compiles at runtime versus precompiled libbpf-tools CO-RE binaries), and drives them from Python: resolve a tool, run it, parse its columns into a summary. Several working examples, plus a minimal BCC program to show what a tool is underneath."
duration: 35 minutes
---

The last stop in the field guide is the toolkit that started it all. **BCC** (the
BPF Compiler Collection) ships roughly a hundred ready-made tracers, and their
names have appeared in nearly every "Cross-check" section of this book —
`execsnoop`, `opensnoop`, `biolatency`, `runqlat`, `tcpconnect`, `profile`. They
are the off-the-shelf instruments you reach for first to confirm what an Aya
program reports, and most are themselves Python, so driving them from Python is
natural: resolve the tool, run it, parse its columns, summarize. This chapter
tours the suite, explains its two generations, and gives several working wrappers
— then shows what a BCC tool actually *is* underneath.

The code is in `examples/66-bcc-tools/`. `./demo.sh` resolves and runs several
tools through a Python summarizer; the `README.md` has the details.

{% include excalidraw.html
   file="bcc-tools"
   alt="Drive the ready-made BCC tools from Python — the validation suite, summarized. A Python wrapper spawns a BCC tool (execsnoop, biolatency, runqlat, tcpconnect — classic Python plus C, or libbpf CO-RE), which attaches to the kernel (compiling first if classic) where probes and maps run; the kernel returns events. The tool prints columnar text on stdout, which the wrapper parses into a top-N summary, counts, or a captured histogram. Classic BCC compiles inline C at runtime and needs clang plus kernel headers; libbpf-tools are precompiled CO-RE from Chapter 58. BCC is for depth, bpftrace for quick questions, Aya for production."
   caption="Figure 66.1 — A BCC tool gathers; the Python wrapper resolves, runs, and summarizes it" %}

## Two generations of BCC

"BCC" means two related things today, and the difference is exactly the CO-RE
story from Chapter 58:

- **Classic BCC** — each tool is a Python script with an **inline C** BPF program
  that BCC **compiles at runtime** with Clang/LLVM against the running kernel's
  headers, then loads. Maximum flexibility, but it carries heavy dependencies:
  `clang`, `llvm`, and **kernel headers matching the running kernel** must be
  present, or you get the infamous `Failed to compile BPF text`. On Fedora these
  live in `bcc-tools`, installed under `/usr/share/bcc/tools/` (on Debian they're
  suffixed `-bpfcc` and on `$PATH`).
- **libbpf-tools** — the same tools rewritten as **precompiled C binaries** using
  libbpf and **CO-RE**: compiled once, they relocate against the target kernel's
  BTF at load time, so they need *no* Clang, no headers, no Python at runtime —
  just the binary. Fedora packages these as `libbpf-tools`. This is the modern
  direction, and the same portability argument that motivates shipping an Aya
  binary rather than compiling on each host.

Knowing which you're running explains both the dependency footprint and why the
libbpf-tools version starts faster and survives a header-less production image.

## The suite, grouped

A working map of the tools you'll actually reach for, several of which mirror
chapters you built in Aya:

| Area | Tools | Mirrors |
|---|---|---|
| Process / syscall | `execsnoop`, `opensnoop`, `statsnoop`, `syscount`, `killsnoop` | Ch 9, 11, 38 |
| Scheduler / CPU | `runqlat`, `runqlen`, `profile`, `offcputime` | Ch 21, 23 |
| Block / filesystem | `biolatency`, `biosnoop`, `cachestat`, `filetop`, `*slower` | Ch 24, 25 |
| Network | `tcpconnect`, `tcpaccept`, `tcplife`, `tcpretrans` | Ch 27, 28 |
| Memory / generic | `memleak`, `funccount`, `funclatency`, `stackcount`, `trace`, `argdist` | Ch 18, 23 |

The generic four — `funccount`, `funclatency`, `stackcount`, `trace`,
`argdist` — are worth singling out: they take a probe specification on the
command line, so they're a quick way to ask almost any "how often / how slow /
with what arguments" question without writing a program at all.

The full, current list lives upstream at the project's tools index,
[github.com/iovisor/bcc](https://github.com/iovisor/bcc) (`README.md` and the
`tools/` directory). Mapped onto the kernel stack, the suite looks like this —
each tool drawn at the layer it observes:

{% include excalidraw.html
   file="bcc-tools-map"
   alt="A curated subset of the bcc tools mapped to the kernel layer each observes. Around the stack — applications, system libraries, system call interface, VFS/sockets/scheduler, file systems/TCP-UDP/virtual memory, block/net devices, device drivers — sit tools: filetop and vfsstat and cachestat and opensnoop and statsnoop at VFS; ext4slower and xfsslower at file systems; btrfsslower and nfsslower at the volume manager; biolatency and biosnoop and biotop and bitesize at the block device; bashreadline at applications; mysqld_qslower and dbstat at applications; execsnoop and opensnoop and syscount at the syscall interface; gethostlatency and sslsniff and memleak at system libraries; syscount and killsnoop and statsnoop at the syscall interface; runqlat and cpudist and runqlen and profile and offcputime at the scheduler; oomkill and memleak and slabratetop at virtual memory; tcpconnect and tcpaccept and tcplife and tcpretrans at TCP/UDP; and capable, funccount, funclatency, trace, argdist, and stackcount as general tools."
   caption="Figure 66.2 — a curated subset of the bcc tools, mapped to where each attaches (iovisor/bcc)" %}

## Driving them from Python

Unlike bpftrace and bpftool, BCC tools emit **columnar text**, not JSON — so the
Python pattern is *resolve, run, parse columns*. The example's
`examples/66-bcc-tools/bcc_runner.py` does exactly that, and it's several working
examples in one — run `--list` to see what it knows:

| Wrapper mode | Tools | Summarized as |
|---|---|---|
| parsed → top-N | `execsnoop` | execs per command |
| | `opensnoop`, `statsnoop` | most-opened paths |
| | `tcpconnect`, `tcpaccept`, `tcplife` | busiest remote `host:port` |
| | `killsnoop` | signals per sender |
| | `syscount` | calls per syscall |
| captured as-is | `biolatency`, `runqlat`, `profile`, `biotop`, `cachestat`, … | the tool's own histogram/table |
| run (output captured) | any other, e.g. `funccount 'vfs_*'`, `argdist`, `trace` | raw output |

Behind that table the wrapper does three small things — **resolve**, **run**,
**parse**:

- It **resolves** a tool across the layouts above: an explicit path, then
  `/usr/share/bcc/tools/`, then `$PATH`, then the `-bpfcc` suffix — so the same
  command works on Fedora and Debian.
- It **runs** the tool for a duration, then sends `SIGINT` so summarizing tools
  (the histogram ones) flush their final table.
- It **parses** the tools it knows into a top-N summary: `execsnoop` → execs per
  command, `opensnoop` → most-opened paths, `tcpconnect` → busiest
  destination `host:port`. For tools it doesn't have a parser for — `biolatency`,
  `runqlat`, `profile` — it captures and prints their own summary (already a
  histogram), so *every* tool is runnable through one wrapper.

```python
def resolve(tool):
    for d in ("/usr/share/bcc/tools", "/sbin", "/usr/sbin"):
        p = os.path.join(d, tool)
        if os.path.exists(p):
            return p
    return shutil.which(tool) or shutil.which(tool + "-bpfcc")
```

The value is the same as the previous two chapters: a one-off command becomes a
repeatable check you can point at a service, summarize, and compare against your
Aya program's own numbers — `tcpconnect`'s destination tally beside your
`tcpconnlat` metric, `biolatency`'s histogram beside your `biopattern` one.

## What a BCC tool is underneath

To demystify the suite, the example includes `examples/66-bcc-tools/hello_bcc.py` — a complete BCC
program in a dozen lines, using the `bcc` Python library directly:

```python
from bcc import BPF
b = BPF(text=r'int hello(void *ctx){ bpf_trace_printk("clone\n"); return 0; }')
b.attach_kprobe(event=b.get_syscall_fnname("clone"), fn_name="hello")
b.trace_print()
```

That `BPF(text=...)` call is the whole story: BCC takes the C string, compiles it
with Clang **right then**, loads it, and attaches it — the runtime compilation
that makes classic BCC flexible and dependency-heavy. Set it beside a Chapter 6
Aya program and the contrast is the book in miniature: BCC compiles C at runtime
from Python; Aya compiles Rust ahead of time into one binary you ship. Both ride
the same kernel verifier and maps; they differ in *when* and *in what language*
the program is written and built.

## When to use which

The field guide's through-line: **BCC** for ready-made depth and quick custom
probes (`trace`, `argdist`) when you accept its dependencies; **bpftrace**
(Chapter 64) for fast one-liners in a small DSL; **bpftool** (Chapter 65) to
inspect what's loaded; and **Aya** for the typed, embeddable, single-binary
programs you operate in production. You reach for the first three to *explore and
validate*, and Aya to *ship* — which is exactly how this book used them.

## Build, deploy, observe

```bash
cd examples/66-bcc-tools && ./demo.sh
```

The demo resolves the tools on the VM, runs `execsnoop` and `tcpconnect` through
the summarizer (printing per-command and per-destination tallies), captures a
`biolatency` histogram, and runs `hello_bcc.py` to show runtime compilation in
action. These are terminal tools, so there's no Grafana panel — though the
wrapper's parsed summaries are exactly what you'd forward as `ebpf_*` metrics if
you wanted them on a dashboard.

## Cross-check

```bash
[vm]$ ls /usr/share/bcc/tools | head                    # the installed suite
[vm]$ sudo /usr/share/bcc/tools/execsnoop                # run one directly
[vm]$ sudo /usr/share/bcc/tools/biolatency 5 1           # one 5-second histogram
```

Running a tool directly and seeing the same rows your wrapper parsed confirms the
summarizer is a thin layer over the suite — the same trust boundary as the
previous two chapters.

## What you learned

- **BCC** ships ~100 ready-made tracers (the `*snoop`/`*latency` tools named in
  every cross-check) in two generations: **classic** (Python + inline C,
  runtime-compiled — needs Clang + kernel headers, in `/usr/share/bcc/tools/`)
  and **libbpf-tools** (precompiled **CO-RE** binaries, no runtime deps).
- They emit **columnar text**, so a Python wrapper *resolves, runs, and parses*
  them into summaries — `execsnoop`/`opensnoop`/`tcpconnect` tallies, captured
  histograms — turning the suite into repeatable, comparable checks.
- A BCC tool is **inline C compiled at runtime** (`BPF(text=...)`); the contrast
  with Aya's ahead-of-time Rust binary is the whole book in miniature — same
  kernel, different *when* and *what language*. Use BCC/bpftrace/bpftool to
  explore and validate, Aya to ship.

That closes the field guide, and the technical body of the book. The final
chapter is a retrospective: the whole arc from a kprobe counting `unlink` to
operating a fleet, what carried through, and where eBPF and Aya go next.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that `bcc-tools` installs tools under
`/usr/share/bcc/tools/` and they run (classic BCC needs `clang`, `llvm`, and
kernel headers matching `uname -r`); that `python3-bcc` is present for
`hello_bcc.py`; that the column layouts the parsers assume match your tool
versions (they vary — the wrapper falls back to printing raw output); and
consider `libbpf-tools` where runtime compilation isn't wanted.*
