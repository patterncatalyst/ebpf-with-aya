//! Shared types for the btf-uprobe chapter.
//!
//! `Order` is the struct the target function takes a pointer to. We define it
//! ONCE here and the target app, the eBPF program, and user space all use it —
//! so by construction they agree on the layout. That's the convenient case.
//!
//! The chapter's real lesson: when you DON'T control the target (can't share
//! this definition), BTF is how you recover the layout — dump the target's
//! BTF, generate a #[repr(C)] mirror from it, and (with user-space CO-RE)
//! relocate field offsets so your probe survives the struct changing.
#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Order {
    pub id: u64,
    pub amount_cents: u64,
    pub status: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OrderEvent {
    pub pid: u32,
    pub order: Order,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for OrderEvent {}
