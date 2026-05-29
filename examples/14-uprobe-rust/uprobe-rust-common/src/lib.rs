//! Shared event: a single observed argument to compute().
#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ArgEvent {
    pub pid: u32,
    pub arg0: u64,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ArgEvent {}
