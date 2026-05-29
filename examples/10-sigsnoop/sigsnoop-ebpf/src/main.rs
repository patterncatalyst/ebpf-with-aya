//! sigsnoop-ebpf — tracepoint on syscalls:sys_enter_kill.
//!
//! Fires whenever a process calls kill(2). We record the sender (current
//! pid/comm) plus the target pid and signal number read from the tracepoint
//! args. A single tracepoint, no entry/exit correlation — simpler than
//! opensnoop, to show the minimal per-event pattern.
//!
//! Offsets from: cat /sys/kernel/tracing/events/syscalls/sys_enter_kill/format
//!   pid_t pid  @ 16    int sig @ 24
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid},
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};
use aya_log_ebpf::info;
use sigsnoop_common::SignalEvent;

const KILL_PID_OFF: usize = 16;
const KILL_SIG_OFF: usize = 24;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[tracepoint]
pub fn sys_enter_kill(ctx: TracePointContext) -> u32 {
    let _ = try_kill(&ctx);
    0
}

fn try_kill(ctx: &TracePointContext) -> Result<(), i64> {
    let target_pid = unsafe { ctx.read_at::<i64>(KILL_PID_OFF)? } as i32;
    let sig = unsafe { ctx.read_at::<i64>(KILL_SIG_OFF)? } as i32;

    if let Some(mut slot) = EVENTS.reserve::<SignalEvent>(0) {
        let ev = SignalEvent {
            sender_pid: (bpf_get_current_pid_tgid() >> 32) as u32,
            target_pid,
            sig,
            comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    info!(ctx, "kill sig {} -> pid {}", sig, target_pid);
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
