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

**Unverified.** Confirm `/sys/fs/bpf` is mounted; the Aya pinning API
(`HashMap::pinned` + `map_pin_path`, `take_link`, `FdLink::try_from`/`pin`,
`MapData::from_pin`); that the program keeps counting after the loader exits;
and that removing the pins detaches it.
