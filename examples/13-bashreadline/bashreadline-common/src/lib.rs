//! Shared event for bashreadline: the line a user typed at an interactive bash
//! prompt, plus who typed it.
#![no_std]
pub const COMM_LEN: usize = 16;
pub const LINE_LEN: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ReadlineEvent {
    pub pid: u32,
    pub uid: u32,
    pub comm: [u8; COMM_LEN],
    pub line: [u8; LINE_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ReadlineEvent {}
