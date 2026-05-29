//! btf-uprobe-ebpf — a uprobe on process_order(const Order *).
//!
//! The argument is a POINTER to a struct, not a scalar. To read the struct we:
//!   1. take arg0 as a user pointer,
//!   2. bpf_probe_read_user the whole Order out of the target's memory into a
//!      #[repr(C)] mirror we both agree on.
//!
//! The mirror layout is the crux: here we share `Order` via the common crate,
//! so it's correct by construction. When you can't share it, you generate the
//! mirror from the target's BTF (see the chapter) so the field offsets match.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_probe_read_user},
    macros::{map, uprobe},
    maps::RingBuf,
    programs::ProbeContext,
};
use aya_log_ebpf::info;
use btf_uprobe_common::{Order, OrderEvent};

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[uprobe]
pub fn process_order_enter(ctx: ProbeContext) -> u32 {
    let _ = try_process(&ctx);
    0
}

fn try_process(ctx: &ProbeContext) -> Result<(), i64> {
    // arg0 is `const Order *` in the target's address space.
    let order_ptr: *const Order = ctx.arg(0).ok_or(0i64)?;
    // Copy the whole struct out of user memory into our mirror.
    let order: Order = unsafe { bpf_probe_read_user(order_ptr).map_err(|_| 1i64)? };

    if let Some(mut slot) = EVENTS.reserve::<OrderEvent>(0) {
        let ev = OrderEvent {
            pid: (bpf_get_current_pid_tgid() >> 32) as u32,
            order,
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    info!(ctx, "order id {} amount {}", order.id, order.amount_cents);
    Ok(())
}

#[link_section = "license"]
#[no_mangle]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
