# 64 · bpftrace from Python: one-liners into tools

bpftrace is the "awk for the kernel" we used for cross-checks all book. With
`-f json` it emits **NDJSON**, so a small Python wrapper turns one-liners into
repeatable, parseable tools.

## Pieces

- `bpftrace_tool.py` — stdlib-only wrapper. Runs a `.bt` file (`--program`) or an
  inline program (`-e`), parses the NDJSON stream by type, and renders streams
  (`printf`), tables, or histograms. `--list` shows the bundled programs.
- `programs/` — runnable programs, each mirroring an earlier chapter:
  - streams: `opensnoop.bt` (Ch 9), `execsnoop.bt` (Ch 11), `killsnoop.bt` (Ch 38)
  - tables: `syscount.bt`, `profile.bt` (Ch 23), `vfsstat.bt`, `tcpconnect.bt` (Ch 27)
  - histograms: `readsize.bt` (Ch 24), `runqlat.bt` (Ch 21)

## Run it

```bash
./demo.sh                                              # list + run three on the VM
# on the VM (bpftrace needs sudo):
sudo python3 bpftrace_tool.py --list
sudo python3 bpftrace_tool.py --program programs/runqlat.bt --duration 6
sudo python3 bpftrace_tool.py --program programs/opensnoop.bt
sudo python3 bpftrace_tool.py -e 'tracepoint:syscalls:sys_enter_openat { @[comm]=count(); }
                                  interval:s:1 { print(@); clear(@); }'
```

## bpftrace vs Aya

bpftrace = fast exploration/validation (no compile/deploy). Aya = production
(typed, embeddable, one binary). Use bpftrace to find the signal, Aya to ship it.

## Verification status

**Unverified.** Confirm `bpftrace -f json` NDJSON shapes for your version
(`bpftrace --version`); that each program attaches (`bpftrace -l`); and that
`python3` + `bpftrace` are present on the VM. Tracepoint arg names
(`args.filename`, `args.next_pid`, `args.sig`) can vary by kernel — adjust if a
program won't attach.
