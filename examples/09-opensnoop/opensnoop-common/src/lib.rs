//! Shared event for opensnoop. #[repr(C)]; the kernel writes it, user space
//! reads it. `ret` is the fd on success or a negative errno on failure.
#![no_std]

pub const COMM_LEN: usize = 16;
pub const NAME_LEN: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OpenEvent {
    pub pid: u32,
    pub uid: u32,
    pub ret: i32,
    pub flags: i32,
    pub comm: [u8; COMM_LEN],
    pub filename: [u8; NAME_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for OpenEvent {}
