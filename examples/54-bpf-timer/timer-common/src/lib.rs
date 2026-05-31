#![no_std]

/// One aggregation slot. `timer` is an opaque `struct bpf_timer` (two u64s) the
/// kernel manages; we never touch its bytes from user space.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Slot {
    pub count: u64,     // events this window (bumped by the tracepoint)
    pub rate: u64,      // events in the last second (snapshotted by the timer)
    pub timer: [u64; 2] // struct bpf_timer
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for Slot {}
