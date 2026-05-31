# 58 · CO-RE: compile once, run everywhere

Kernel struct layouts change across versions, so baked-in field offsets break.
**CO-RE** compiles once against generic types, records relocations naming the
fields you touch, and patches offsets at load from the target kernel's **BTF**
(kernels ≥ 5.8 with `CONFIG_DEBUG_INFO_BTF`).

## Pieces

- `reference/core.bpf.c` — canonical C: `BPF_CORE_READ`, a nested read, and
  `bpf_core_field_exists(task->loginuid)` (offset + existence relocations).
- `core-ebpf` — Aya rendering. Aya does CO-RE transparently **when the bindings
  carry BTF**: regenerate `src/vmlinux.rs` with `aya-tool generate task_struct`
  (the committed file is a placeholder).
- `core` — loader; relocations resolve against this kernel's BTF at load.
  Exports `ebpf_core_reads_total`.

## Run it

```bash
aya-tool generate task_struct > core-ebpf/src/vmlinux.rs   # real CO-RE bindings (needs bpftool + bindgen)
./demo.sh
```

## Cross-check

```bash
ls /sys/kernel/btf/vmlinux                                 # target BTF present
llvm-objdump -r core.o | grep -i core                       # relocation records (descriptions, not offsets)
sudo bpftool prog show                                      # loaded program (offsets patched)
```

## Verification status

**Unverified.** Confirm `/sys/kernel/btf/vmlinux` exists; that `aya-tool
generate` produces bindings (needs `bpftool` + `bindgen`); that field reads
resolve and the existence check branches; and that CO-RE relocation records
appear in the object. Kernels without BTF need BTFHub/external BTF.
