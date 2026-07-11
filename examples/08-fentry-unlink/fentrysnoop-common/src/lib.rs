//! Shared event for fentrysnoop. Same #[repr(C)] contract as Chapter 7, plus a
//! `ret` field carrying vfs_unlink's return value (0 = success, negative errno
//! = failure) — captured at fexit, which is the fentry/fexit advantage this
//! chapter demonstrates.
#![no_std]

pub const COMM_LEN: usize = 16;
pub const NAME_LEN: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnlinkEvent {
    pub pid: u32,
    pub uid: u32,
    pub ret: i32,
    pub comm: [u8; COMM_LEN],
    pub filename: [u8; NAME_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for UnlinkEvent {}
