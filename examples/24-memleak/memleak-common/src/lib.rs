//! Shared types for memleak: per-outstanding-allocation info, keyed by the
//! returned pointer. When a pointer is freed we delete its entry; whatever
//! remains at the end is outstanding (a candidate leak).
#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AllocInfo {
    pub size: u64,
    pub stackid: i32, // user StackTrace id of the allocation site
    pub pid: u32,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for AllocInfo {}
