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
- `demo.sh` — runs `bpftool gen skeleton -L` on a compiled BPF object to reveal
  the generated syscall/loader program and its embedded data.

## Run it

```bash
# build any example's eBPF object first, e.g.:
( cd ../48-pin-demo && cargo build --release )
./demo.sh                       # uses the pin-demo object by default
BPF_OBJ=/path/to/prog.o ./demo.sh
```

## Cross-check

```bash
bpftool gen skeleton -L program.o | sed -n '1,40p'   # loader program + data
bpftool gen skeleton    program.o | sed -n '1,12p'   # full skeleton, for contrast
```

## Verification status

**Unverified.** Confirm `bpftool gen skeleton -L` emits a syscall/loader
program for the object; the `bpf_sys_bpf`/`bpf_sys_close` helper surface; and
treat `illustrative/loader_program.rs` as a sketch — Aya syscall-program
authoring is experimental.
