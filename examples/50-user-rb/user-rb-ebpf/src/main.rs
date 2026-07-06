#![no_std]
#![no_main]
//! EXPERIMENTAL SKETCH — the Aya rendering of the user-ring-buffer consumer.
//! Aya recognizes BPF_MAP_TYPE_USER_RINGBUF, but the kernel-side drain helper
//! and the dynptr accessor used in the callback are still settling. The
//! canonical form is reference/user_ringbuf.bpf.c. Shape, not turnkey code.

use core::cell::UnsafeCell;

use aya_ebpf::{
    bindings::{bpf_map_def, bpf_map_type::BPF_MAP_TYPE_USER_RINGBUF},
    cty::c_void,
    helpers::bpf_user_ringbuf_drain,
    macros::{map, tracepoint},
    maps::HashMap,
    programs::TracePointContext,
};
use user_rb_common::Sample;

/// Minimal user-ring-buffer map wrapper.
///
/// aya-ebpf 0.1 has no high-level `UserRingBuf` type, so we declare the map def
/// directly (BPF_MAP_TYPE_USER_RINGBUF) and drain it via the raw
/// `bpf_user_ringbuf_drain` helper. The callback runs once per submitted sample.
#[repr(transparent)]
pub struct UserRingBuf {
    def: UnsafeCell<bpf_map_def>,
}

unsafe impl Sync for UserRingBuf {}

impl UserRingBuf {
    pub const fn with_byte_size(byte_size: u32, flags: u32) -> Self {
        Self {
            def: UnsafeCell::new(bpf_map_def {
                type_: BPF_MAP_TYPE_USER_RINGBUF,
                key_size: 0,
                value_size: 0,
                max_entries: byte_size,
                map_flags: flags,
                id: 0,
                pinning: 0,
            }),
        }
    }

    /// Drain all pending samples, invoking `callback` for each one.
    pub fn drain(&self, callback: fn(&Sample) -> u32) -> i64 {
        unsafe {
            bpf_user_ringbuf_drain(
                self.def.get() as *mut c_void,
                callback as *mut c_void,
                core::ptr::null_mut(),
                0,
            )
        }
    }
}

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
