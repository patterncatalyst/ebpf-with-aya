//! Shared event for exitsnoop. `code` is the status passed to exit_group(2):
//! the low 8 bits are the program's exit code on a normal exit.
#![no_std]
pub const COMM_LEN: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExitEvent {
    pub pid: u32,
    pub code: i32,
    pub comm: [u8; COMM_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ExitEvent {}
