# 64 · bpftrace from Python: one-liners into tools

bpftrace is the "awk for the kernel" we used for cross-checks all book. With
`-f json` it emits **NDJSON**, so a small Python wrapper turns a one-liner into a
repeatable, parseable tool.

## Pieces

- `bpftrace_tool.py` — stdlib-only wrapper: runs a `.bt` program with
  `-f json`, parses the stream by event type, renders a top-N table / histogram.
- `programs/syscount.bt` — syscalls per command (a `map` event each second).
- `programs/readsize.bt` — `read()` size histogram (`lhist` buckets).

## Run it

```bash
./demo.sh          # live syscall-top on the VM for 8s
# on the VM:
sudo python3 bpftrace_tool.py --program programs/readsize.bt
```

## bpftrace vs Aya

bpftrace = fast exploration/validation (no compile/deploy). Aya = production
(typed, embeddable, one binary). Use bpftrace to find the signal, Aya to ship it.

## Verification status

**Unverified.** Confirm `bpftrace -f json` NDJSON shapes for your version
(`bpftrace --version`); that the programs attach (`bpftrace -l`); and that
`python3` + `bpftrace` are present on the VM.
