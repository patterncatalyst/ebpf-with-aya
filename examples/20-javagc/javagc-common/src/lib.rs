//! Shared event: one completed GC pause (begin -> end), duration in ns.
#![no_std]
pub const COMM_LEN: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GcEvent {
    pub pid: u32,
    pub _pad: u32,
    pub pause_ns: u64,
    pub comm: [u8; COMM_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for GcEvent {}
