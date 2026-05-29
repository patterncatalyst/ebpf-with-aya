//! Shared map-value type for hardirqs: accumulated time + count per IRQ vector.
#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IrqStat {
    pub count: u64,
    pub total_ns: u64,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for IrqStat {}
