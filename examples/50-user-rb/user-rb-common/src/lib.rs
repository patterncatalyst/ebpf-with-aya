#![no_std]

/// One sample produced by user space and consumed by the BPF program.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sample {
    pub value: u64,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for Sample {}
