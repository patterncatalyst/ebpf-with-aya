//! Shared event for sigsnoop: who sent which signal to whom.
#![no_std]
pub const COMM_LEN: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SignalEvent {
    pub sender_pid: u32,
    pub target_pid: i32,
    pub sig: i32,
    pub comm: [u8; COMM_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for SignalEvent {}
