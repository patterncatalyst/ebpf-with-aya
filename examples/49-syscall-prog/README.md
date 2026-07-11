# 49 · Syscall programs: when BPF loads BPF

`BPF_PROG_TYPE_SYSCALL` programs attach to nothing — you run them via
`BPF_PROG_RUN`, and they call `bpf()` themselves with `bpf_sys_bpf()`. Their
main use is **loader programs**: a "strace of libbpf" replayed in-kernel, which
`bpftool` embeds in a **light skeleton** (no libelf/libbpf at load time) and
which paves the way to **signed** loading.

> **Why no Aya tool here?** Aya *is* the user-space loader, and syscall-program
> authoring in aya-ebpf is experimental. So this chapter is conceptual: an
> inspectable real artifact (the light skeleton) plus an illustrative sketch.

## Contents

- `illustrative/loader_program.rs` — the shape of a loader program (read-only;
  not built).
- `reference/skel_demo.bpf.c` — a tiny libbpf-style C object (one map, one prog)
  compiled on the target so we can generate a light skeleton from it.
- `demo.sh` — compiles that object on the target and runs `bpftool gen skeleton
  -L` to reveal the generated syscall/loader program and its embedded data.

## Why not an aya object?

The obvious idea — point this at an aya example's `.o` — doesn't work:
`aya-ebpf` emits **legacy `maps`-section** map definitions, which libbpf v1.0+
refuses to open (`bpftool` reports *"legacy map definitions in 'maps' section
are not supported"*). Skeletons are a **libbpf** concept; aya is its own loader
and doesn't produce libbpf-loadable objects with BTF-defined maps. So we compile
a small C object (with a `.maps` section) on the target instead.

## Run it

```bash
./demo.sh                          # compiles reference/skel_demo.bpf.c on the VM
BPF_OBJ=/path/to/libbpf-object.o ./demo.sh   # or your own libbpf-compatible object
```

Needs `clang`, `libbpf-devel`, and `bpftool` on the target (Chapter 4 toolchain).

## Cross-check

```bash
bpftool gen skeleton -L program.o | sed -n '1,40p'   # loader program + data
bpftool gen skeleton    program.o | sed -n '1,12p'   # full skeleton, for contrast
```

## Verification status

**Verified — Fedora 44, kernel 7.1.3 (bpftool v7.6.0, libbpf 1.6).** The light
skeleton emits `#include <bpf/skel_internal.h>` + a `struct bpf_loader_ctx`
(the BPF_PROG_TYPE_SYSCALL loader path); the full skeleton emits
`#include <bpf/libbpf.h>` + `struct bpf_object_skeleton` (the classic ELF path).
Treat `illustrative/loader_program.rs` as a sketch — aya syscall-program
authoring is experimental.
