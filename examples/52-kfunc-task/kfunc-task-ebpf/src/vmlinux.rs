// PLACEHOLDER kernel bindings. Regenerate the real, BTF-backed type with:
//   aya-tool generate task_struct > kfunc-task-ebpf/src/vmlinux.rs
// The opaque type below lets the crate compile; the real type (carrying BTF)
// is what lets the kfunc argument resolve correctly at load time. The full
// CO-RE / aya-tool story is Part 9.
#![allow(non_camel_case_types, dead_code)]

#[repr(C)]
pub struct task_struct {
    _opaque: [u8; 0],
}
