#![no_std]
#![no_main]
//! Per-second event rate — the userspace-timed rendering.
//!
//! The canonical reference/timer.bpf.c does the per-second snapshot IN THE
//! KERNEL with a `bpf_timer`: it stores a `struct bpf_timer` in the map value,
//! calls bpf_timer_init/set_callback/start, and a softirq callback snapshots the
//! rate and re-arms. **That doesn't work from aya-ebpf**: the kernel refuses to
//! use bpf_timer unless the map's value BTF declares a field of the kernel type
//! `struct bpf_timer` (`map 'SLOTS' has to have BTF in order to use bpf_timer`),
//! and Rust/aya-ebpf can't emit that opaque kernel type in the value's BTF — a
//! `[u64; 2]` is just two integers, not a `struct bpf_timer`, to the verifier.
//!
//! So this program does the minimal, verifier-safe half — maintain a running
//! count — and the userspace loader samples it once a second and takes the
//! delta. Same observable (events/sec); the periodic work moves from a kernel
//! timer to a userspace tokio timer. See the C reference for the in-kernel form.

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::Array,
    programs::TracePointContext,
};
use timer_common::Slot;

#[map]
static SLOTS: Array<Slot> = Array::with_max_entries(1, 0);

#[tracepoint]
pub fn count(_ctx: TracePointContext) -> u32 {
    if let Some(s) = SLOTS.get_ptr_mut(0) {
        unsafe { (*s).count += 1 };
    }
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
