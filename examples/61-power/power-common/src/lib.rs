#![no_std]

/// A task command name, used as the per-workload bucket key.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Comm {
    pub name: [u8; 16],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for Comm {}
