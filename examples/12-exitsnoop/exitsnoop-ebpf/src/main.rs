//! exitsnoop-ebpf — tracepoint on syscalls:sys_enter_exit_group.
//!
//! exit_group(2) is how a process (all its threads) terminates; glibc calls it
//! from exit()/return-from-main. Its single arg `error_code` carries the exit
//! status, so we get the exit code WITHOUT touching task_struct — keeping this
//! robust across kernels (contrast the libbpf exitsnoop, which reads exit_code
//! and start time from task_struct via CO-RE).
//!
//! Offset from sys_enter_exit_group/format: error_code@16.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid},
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};
use aya_log_ebpf::info;
use exitsnoop_common::ExitEvent;

const CODE_OFF: usize = 16;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[tracepoint]
pub fn sys_enter_exit_group(ctx: TracePointContext) -> u32 {
    let _ = try_exit(&ctx);
    0
}

fn try_exit(ctx: &TracePointContext) -> Result<(), i64> {
    let code = unsafe { ctx.read_at::<i64>(CODE_OFF)? } as i32;
    if let Some(mut slot) = EVENTS.reserve::<ExitEvent>(0) {
        let ev = ExitEvent {
            pid: (bpf_get_current_pid_tgid() >> 32) as u32,
            code,
            comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    info!(ctx, "exit_group code {}", code);
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
