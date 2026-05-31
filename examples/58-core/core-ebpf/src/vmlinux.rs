// PLACEHOLDER bindings. Regenerate the real, CO-RE-portable ones with:
//   aya-tool generate task_struct > src/vmlinux.rs
// The real bindings carry BTF so that reading a field emits a relocation the
// loader patches against the target kernel. This stub will NOT read correct
// offsets without regeneration — that is exactly the point of the chapter.
#![allow(non_camel_case_types, dead_code)]

#[repr(C)]
pub struct task_struct {
    pub pid: i32,
    pub tgid: i32,
    pub comm: [u8; 16],
    // ... aya-tool emits the full struct (+ BTF) here
}
