#![no_std]
#![no_main]
//! A trivial service: count getpid calls into EVENTS. The point of the chapter
//! is operational — this map and the program's link get pinned so they outlive
//! the loader, and the count survives across loader restarts/upgrades.

use aya_ebpf::{macros::{map, tracepoint}, maps::Array, programs::TracePointContext};

#[map] static EVENTS: Array<u64> = Array::with_max_entries(1, 0);

#[tracepoint]
pub fn count(_ctx: TracePointContext) -> u32 {
    if let Some(c) = EVENTS.get_ptr_mut(0) {
        unsafe { *c += 1 };
    }
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
