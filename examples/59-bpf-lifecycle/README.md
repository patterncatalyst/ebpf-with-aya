# 59 · Operating eBPF: lifecycle, pinning, zero-downtime

A BPF object's lifetime is normally its loader's file descriptors. Operating
eBPF means decoupling that with three pillars: **pinning** (lifetime),
**link update** (zero-downtime swap), **pinned maps** (state continuity).

## What this shows

- `lifecycle-ebpf` — a counter program (getpid → `EVENTS`).
- `lifecycle` — loads it, **pins the map** (`EbpfLoader::map_pin_path`) and the
  **link** (`FdLink::pin`), drives events, exits leaving them pinned. Run again
  and it **reuses** the pinned map: the count continues. Exports
  `ebpf_service_events_total`.

## Run it

```bash
./demo.sh          # run 1 pins + exits; the program keeps counting; run 2 reuses the state
./demo.sh build
```

## Cross-check

```bash
ls -l /sys/fs/bpf/ebpf-aya/                          # pinned link + map
sudo bpftool link show                                # attachment with no owning process
sudo bpftool map dump pinned /sys/fs/bpf/ebpf-aya/EVENTS   # value climbing after the loader exits
```

## At fleet scale

- **L3AF** (LF Networking): full lifecycle, program chaining, graceful restart.
- **bpfman** (CNCF): Kubernetes-native eBPF management and pinning.

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab VM:
`map_pin_path` + `FdLink::pin` pin to bpffs, the program keeps running and
`EVENTS` keeps updating after the loader exits, and a second run reuses the
pinned map and continues the count. Attach targets and struct offsets can be
kernel-version-specific, and `link_update` atomic-swap ergonomics in Aya are
still evolving.
