#![no_std]
#![no_main]
//! EXPERIMENTAL SKETCH — the Aya rendering of the user-ring-buffer consumer.
//! Aya recognizes BPF_MAP_TYPE_USER_RINGBUF, but the kernel-side drain helper
//! and the dynptr accessor used in the callback are still settling. The
//! canonical form is reference/user_ringbuf.bpf.c. Shape, not turnkey code.

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::{HashMap, UserRingBuf},
    programs::TracePointContext,
};
use user_rb_common::Sample;

// user-space is the producer; this program is the consumer
#[map] static USER_RB: UserRingBuf = UserRingBuf::with_byte_size(256 * 1024, 0);
// aggregate built by the callback: key 0 = count, key 1 = sum
#[map] static AGG: HashMap<u32, u64> = HashMap::with_max_entries(2, 0);

#[inline(always)]
fn bump(key: u32, by: u64) {
    let n = unsafe { AGG.get(&key).copied().unwrap_or(0) } + by;
    let _ = AGG.insert(&key, &n, 0);
}

// callback: invoked once per submitted sample (arrives as a dynptr)
fn on_sample(sample: &Sample) -> u32 {
    bump(0, 1);
    bump(1, sample.value);
    0
}

#[tracepoint]
pub fn drain_it(_ctx: TracePointContext) -> u32 {
    // drain everything pending, invoking on_sample per sample
    let _ = USER_RB.drain(on_sample);
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
