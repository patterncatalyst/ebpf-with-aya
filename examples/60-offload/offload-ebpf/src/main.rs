#![no_std]
#![no_main]
//! A packet-counting XDP program (XDP_PASS — safe on a live interface). The same
//! object can attach in generic, native, or offload mode; where it runs (host
//! CPU vs the NIC) is decided by the loader's attach flag, not by this code.

use aya_ebpf::{bindings::xdp_action, macros::{map, xdp}, maps::Array, programs::XdpContext};

#[map] static PKTS: Array<u64> = Array::with_max_entries(1, 0);

#[xdp]
pub fn count(_ctx: XdpContext) -> u32 {
    if let Some(c) = PKTS.get_ptr_mut(0) {
        unsafe { *c += 1 };
    }
    xdp_action::XDP_PASS
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
