#![no_std]

/// One aggregation slot: a running event count that the userspace loader samples
/// once a second to compute the per-second rate.
///
/// The canonical design (reference/timer.bpf.c) keeps a `struct bpf_timer` in
/// this value and snapshots the rate *in the kernel*. We can't do that from
/// aya-ebpf — see the note in the loader — so the value is just the count.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Slot {
    pub count: u64, // events so far (bumped by the tracepoint)
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for Slot {}
