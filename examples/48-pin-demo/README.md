# 48 · Detaching and pinning: outliving the loader

BPF programs, maps, and links are **reference-counted** — when the loader's fds
close, they're freed. **Pinning to bpffs** (`/sys/fs/bpf`) creates a named
reference that keeps them alive past the loader.

## Pieces

- `pin-demo-ebpf` — a `sys_enter_execve` tracepoint counting execs into a map
  declared `HashMap::pinned(...)` (LIBBPF_PIN_BY_NAME).
- `pinctl` — `load` (attach + pin link & map, then exit), `read` (open the
  pinned map from a fresh process; export `ebpf_pinned_execs_total`), `detach`
  (remove the pins).

## Run it

```bash
./demo.sh          # load+pin (exit) -> show persistence -> read x2 -> detach
./demo.sh build    # just build pinctl on the host
```

## Cross-check

```bash
sudo bpftool prog show          # program still loaded with no loader running
sudo bpftool link show          # the pinned link holds the attachment
sudo bpftool map dump pinned /sys/fs/bpf/ebpf-aya/EXECS
ls -l /sys/fs/bpf/ebpf-aya/
```

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab
VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches, and pins the
link and map to bpffs; the program keeps counting execs after the loader exits,
a fresh `read` process opens the pinned map, and `detach` removes the pins.
Attach targets and struct offsets can be kernel-version-specific.
