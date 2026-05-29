//! goroutine-ebpf — a UPROBE on Go's runtime.casgstatus.
//!
//! Go's scheduler calls runtime.casgstatus(gp *g, oldval, newval uint32) on
//! EVERY goroutine state transition. Probing its entry lets us observe the
//! scheduler's state machine.
//!
//! TWO Go-specific gotchas:
//!  1. REGISTER ABI. Go 1.17+ uses its own register ABI (ABIInternal), NOT the
//!     C ABI. Integer/pointer args go in RAX, RBX, RCX, RDI, RSI, R8, R9, ...
//!     So newval (the 3rd arg) is in RCX — aya's ctx.arg(2) would read the C-ABI
//!     register (RDX) and be WRONG. We read RCX from pt_regs directly.
//!  2. NO URETPROBES on Go. Go grows/moves goroutine stacks; uretprobe return
//!     trampolines can corrupt them and crash the program. Use uprobes only.
#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::pt_regs,
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid},
    macros::{map, uprobe},
    maps::RingBuf,
    programs::ProbeContext,
};
use aya_log_ebpf::info;
use goroutine_common::GoStateEvent;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

#[uprobe]
pub fn casgstatus(ctx: ProbeContext) -> u32 {
    let _ = try_cas(&ctx);
    0
}

fn try_cas(ctx: &ProbeContext) -> Result<(), i64> {
    // Read RCX directly: Go ABIInternal puts the 3rd integer arg (newval) there.
    // pt_regs field names follow the kernel/x86_64 layout (verify on your build).
    let regs = ctx.as_ptr() as *const pt_regs;
    if regs.is_null() {
        return Err(0);
    }
    let newstate = unsafe { (*regs).rcx } as u32;

    if let Some(mut slot) = EVENTS.reserve::<GoStateEvent>(0) {
        let ev = GoStateEvent {
            pid: (bpf_get_current_pid_tgid() >> 32) as u32,
            newstate,
            comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    info!(ctx, "goroutine -> state {}", newstate);
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
