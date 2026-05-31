#![no_std]
#![no_main]
//! EXPERIMENTAL SKETCH — the Aya rendering of the canonical reference/timer.bpf.c.
//! The bpf_timer lifecycle (init/set_callback/start) maps directly to helpers,
//! but expressing the callback as a verifier-checkable subprogram in aya-ebpf is
//! still rough. The C reference is authoritative; confirm the helper names and
//! the callback mechanics against your aya-ebpf version.

use aya_ebpf::{
    helpers::{bpf_timer_init, bpf_timer_set_callback, bpf_timer_start},
    macros::{map, tracepoint},
    maps::Array,
    programs::TracePointContext,
};
use timer_common::Slot;

#[map] static SLOTS: Array<Slot> = Array::with_max_entries(1, 0);

const CLOCK_MONOTONIC: u64 = 1;
const NSEC_PER_SEC: u64 = 1_000_000_000;

// runs in softirq every second; snapshots the window and re-arms
unsafe extern "C" fn tick(_map: *mut core::ffi::c_void, _key: *mut u32, val: *mut Slot) -> i32 {
    if !val.is_null() {
        (*val).rate = (*val).count;
        (*val).count = 0;
        let t = &mut (*val).timer as *mut _ as *mut _;
        bpf_timer_start(t, NSEC_PER_SEC, 0);
    }
    0
}

#[tracepoint]
pub fn count(_ctx: TracePointContext) -> u32 {
    if let Some(s) = SLOTS.get_ptr_mut(0) {
        unsafe { (*s).count += 1 };
    }
    0
}

#[tracepoint]
pub fn arm(_ctx: TracePointContext) -> u32 {
    if let Some(s) = SLOTS.get_ptr_mut(0) {
        unsafe {
            let t = &mut (*s).timer as *mut _ as *mut _;
            bpf_timer_init(t, &SLOTS as *const _ as *mut _, CLOCK_MONOTONIC);
            bpf_timer_set_callback(t, tick as *mut _);
            bpf_timer_start(t, NSEC_PER_SEC, 0);
        }
    }
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
