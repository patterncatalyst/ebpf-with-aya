//! Shared key for the profiler's count map: a (pid, comm, kernel-stack-id,
//! user-stack-id) tuple. Equal stacks hash to the same StackTrace id, so this
//! key collapses identical samples and the value is just the sample count.
#![no_std]
pub const COMM_LEN: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StackKey {
    pub pid: u32,
    pub kstack: i32, // StackTrace id, or -1 if none
    pub ustack: i32, // StackTrace id, or -1 if none
    pub comm: [u8; COMM_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for StackKey {}
