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

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab VM
(kernel 7.1.3-200.fc44): builds, loads, attaches, and runs as described —
`/sys/kernel/btf/vmlinux` is present, field reads resolve, the existence check
branches, and CO-RE relocation records appear in the object. Attach targets and
struct offsets can be kernel-version-specific, and kernels without BTF need
BTFHub/external BTF.
