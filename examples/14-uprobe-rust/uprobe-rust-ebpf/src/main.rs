//! uprobe-rust-ebpf — a UPROBE on the `compute` symbol of a Rust binary.
//!
//! A uprobe fires on function ENTRY, where arguments are available. For a
//! `extern "C" fn compute(x: u64)` the first arg is in the first arg register,
//! which ctx.arg(0) reads. (A mangled Rust fn would need the mangled symbol and
//! Rust's calling convention — extern "C" + no_mangle keeps this simple.)
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::bpf_get_current_pid_tgid,
    macros::{map, uprobe},
    maps::RingBuf,
    programs::ProbeContext,
};
use aya_log_ebpf::info;
use uprobe_rust_common::ArgEvent;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[uprobe]
pub fn compute_enter(ctx: ProbeContext) -> u32 {
    let _ = try_compute(&ctx);
    0
}

fn try_compute(ctx: &ProbeContext) -> Result<(), i64> {
    let arg0: u64 = ctx.arg(0).ok_or(0i64)?;
    if let Some(mut slot) = EVENTS.reserve::<ArgEvent>(0) {
        let ev = ArgEvent {
            pid: (bpf_get_current_pid_tgid() >> 32) as u32,
            arg0,
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    info!(ctx, "compute() called with arg0 {}", arg0);
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
