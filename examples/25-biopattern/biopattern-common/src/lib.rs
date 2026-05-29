//! Shared per-device counters for biopattern.
#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BioStat {
    pub sequential: u64,
    pub random: u64,
    pub bytes: u64,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for BioStat {}
