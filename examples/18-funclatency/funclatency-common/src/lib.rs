//! Shared event: one measured function call duration.
#![no_std]
pub const COMM_LEN: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LatEvent {
    pub pid: u32,
    pub _pad: u32,
    pub delta_ns: u64,
    pub comm: [u8; COMM_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for LatEvent {}
