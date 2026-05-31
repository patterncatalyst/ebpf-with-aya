#![no_std]

pub const KIND_QUERY: u32 = 0;
pub const KIND_LOCK: u32 = 1;

/// One observed event from a postgres backend: a completed query (with up to
/// 128 bytes of SQL text) or a finished lock wait. Keyed in-kernel by the
/// backend pid; the pid travels in the event for attribution.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Event {
    pub kind: u32,
    pub pid: u32,
    pub dur_ns: u64,
    pub query: [u8; 128],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for Event {}
