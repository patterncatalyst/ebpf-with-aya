//! Event record shared between the kernel program and user space.
//!
//! #[repr(C)] so the bytes the kernel writes into the ring buffer line up
//! exactly with what user space reads back. This is the contract; get a field
//! order or size wrong and user space reads garbage.
#![no_std]

pub const COMM_LEN: usize = 16;
pub const NAME_LEN: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnlinkEvent {
    pub pid: u32,
    pub uid: u32,
    /// Process name (bpf_get_current_comm), NUL-padded.
    pub comm: [u8; COMM_LEN],
    /// Best-effort filename read from the kprobe argument. May be empty if the
    /// read failed (see the chapter — reading kernel struct fields is the
    /// version-sensitive part).
    pub filename: [u8; NAME_LEN],
}

// SAFETY: plain old data, no padding-sensitive invariants. Allows the user
// crate to treat received bytes as this type.
#[cfg(feature = "user")]
unsafe impl aya::Pod for UnlinkEvent {}
