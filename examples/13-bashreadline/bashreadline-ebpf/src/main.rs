//! bashreadline-ebpf — a URETPROBE on bash's readline().
//!
//! readline() returns a `char *` to the line the user just typed at an
//! interactive prompt. A uretprobe fires on the function's RETURN, where the
//! return value is available, so we read that pointer (USER memory of the bash
//! process) into our event.
//!
//! This is a USER-space probe (uprobe family) — attaches to a function in a
//! binary/library, not the kernel. Contrast everything in Part "Tracing the
//! kernel".
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid, bpf_probe_read_user_str_bytes},
    macros::{map, uretprobe},
    maps::RingBuf,
    programs::RetProbeContext,
};
use aya_log_ebpf::info;
use bashreadline_common::{ReadlineEvent, LINE_LEN};

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[uretprobe]
pub fn readline_ret(ctx: RetProbeContext) -> u32 {
    let _ = try_readline(&ctx);
    0
}

fn try_readline(ctx: &RetProbeContext) -> Result<(), i64> {
    // Return value of readline(): char * to the typed line (user memory).
    let line_ptr: *const u8 = ctx.ret();
    if line_ptr.is_null() {
        return Ok(());
    }

    if let Some(mut slot) = EVENTS.reserve::<ReadlineEvent>(0) {
        let ev = slot.as_mut_ptr();
        unsafe {
            (*ev).pid = (bpf_get_current_pid_tgid() >> 32) as u32;
            (*ev).uid = (bpf_get_current_uid_gid() & 0xffff_ffff) as u32;
            (*ev).comm = bpf_get_current_comm().unwrap_or([0u8; 16]);
            (*ev).line = [0u8; LINE_LEN];
            let _ = bpf_probe_read_user_str_bytes(line_ptr, &mut (*ev).line);
        }
        slot.submit(0);
    }
    info!(ctx, "bash readline captured");
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
