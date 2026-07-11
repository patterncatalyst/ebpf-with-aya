# 66 · The BCC tools: the ready-made suite

BCC ships ~100 ready-made tracers (the `*snoop`/`*latency` tools we cross-checked
with all book). Two generations: **classic** (Python + inline C, runtime-compiled
via Clang — in `/usr/share/bcc/tools/`) and **libbpf-tools** (precompiled CO-RE
binaries). They print columnar text, so a Python wrapper resolves/runs/parses them.

## Pieces

- `bcc_runner.py` — `--list` shows what it knows; resolve a tool (Fedora `/usr/share/bcc/tools`, `$PATH`, or
  `-bpfcc`), run it for a duration, parse columns into a top-N summary
  (`execsnoop`/`opensnoop`/`statsnoop`/`tcpconnect`/`tcpaccept`/`tcplife`/`killsnoop`/`syscount`), or capture+print a tool's own histogram (`biolatency`/`runqlat`/`profile`/`biotop`/`cachestat`).
- `hello_bcc.py` — a minimal BCC-library program: inline C compiled at runtime
  (`BPF(text=...)`), the contrast with Aya's ahead-of-time Rust binary.

## Run it

```bash
./demo.sh                              # several tools via the summarizer on the VM
# on the VM (sudo):
sudo python3 bcc_runner.py execsnoop
sudo python3 bcc_runner.py tcpconnect --duration 10
sudo python3 bcc_runner.py biolatency 5 1
sudo python3 hello_bcc.py              # needs python3-bcc + clang + kernel headers
```

## When to use which

BCC = ready-made depth + quick custom probes (`trace`, `argdist`); bpftrace =
fast one-liners; bpftool = inspect what's loaded; Aya = production. Explore/validate
with the first three; ship with Aya.

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Run on the lab VM (Fedora 44, kernel
7.1.3-200.fc44): `bcc-tools` installs under `/usr/share/bcc/tools/`, the tools
resolve and run through the Python wrapper, and `hello_bcc.py` compiles and
attaches via `python3-bcc`. Classic BCC needs `clang`, `llvm`, and kernel headers
matching `uname -r`; column layouts and attach targets can be version-specific, so
parsers fall back to raw output where a tool's format differs.
