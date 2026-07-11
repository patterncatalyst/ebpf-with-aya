# 56 · Dynptrs and arenas: flexible memory for BPF

Two ways BPF escapes fixed-size memory. A **dynptr** (kernel ≥ 5.19) is a
verifier-tracked handle to a variable-length region. A **BPF arena**
(kernel ≥ 6.9) is a sparse region shared zero-copy with user space where BPF
builds real pointer-based data structures.

## Pieces

- `reference/dynptr_ringbuf.bpf.c` — canonical: reserve exactly `len` bytes via
  a ring-buffer dynptr, fill, submit (true variable length).
- `reference/arena_list.bpf.c` — canonical: a linked list in a BPF arena with
  real `__arena` pointers; user space mmaps it.
- `dynptr-ebpf` — Aya rendering (reserves a fixed Record + logical `len`; aya
  dynptr reserve is emerging).
- `dynptr-common` / `dynptr` — the `Record`, and a loader that reads the
  variable-length records; exports `ebpf_dynptr_records_total`.

## Run it

```bash
./demo.sh          # read variable-length records; then compile+load the arena list
./demo.sh build
```

## Cross-check

```bash
sudo bpftool map show | grep -E 'ringbuf|arena'
sudo bpftool map dump name arena | head        # arena bytes (if built)
```

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab VM
(Fedora 44, kernel 7.1.3-200.fc44): the dynptr ring-buffer producer and aya
rendering build, load, and attach cleanly, variable-length records arrive
intact, and the arena example compiles with `-D__BPF_FEATURE_ADDR_SPACE_CAST`
and loads (kernel 7.1.3 satisfies the ≥ 5.19 dynptr and ≥ 6.9 arena floors).
aya dynptr/arena support is emerging, so the C references remain canonical;
attach targets and struct offsets can be kernel-version-specific.
