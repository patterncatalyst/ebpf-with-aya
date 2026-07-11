//! profile-ebpf — a sampling CPU profiler.
//!
//! A perf_event program fires on a timer (set up at 99 Hz per CPU in user
//! space). On each tick it captures the CURRENT call stack — kernel and user —
//! into a StackTrace map via bpf_get_stackid, and bumps a per-stack counter.
//! Sampling means cost is fixed (99/sec/CPU) regardless of how busy the box is.
#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::BPF_F_USER_STACK,
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid},
    macros::{map, perf_event},
    maps::{HashMap, StackTrace},
    programs::{PerfEventContext, tracing::StackIdContext},
};
use profile_common::StackKey;

// StackTrace stores the actual frame addresses; COUNTS maps a stack tuple ->
// sample count. Both sized generously for a short profile run.
#[map]
static STACKS: StackTrace = StackTrace::with_max_entries(16384, 0);
#[map]
static COUNTS: HashMap<StackKey, u64> = HashMap::with_max_entries(16384, 0);

#[perf_event]
pub fn profile_cpu(ctx: PerfEventContext) -> u32 {
    let id = bpf_get_current_pid_tgid();
    let pid = (id >> 32) as u32;
    if pid == 0 {
        return 0; // skip the idle task
    }

    // Capture kernel and user stacks. Negative => unavailable for this sample.
    let kstack = ctx.get_stackid(&STACKS, 0).unwrap_or(-1) as i32;
    let ustack = ctx.get_stackid(&STACKS, BPF_F_USER_STACK as u64).unwrap_or(-1) as i32;

    let key = StackKey {
        pid,
        kstack,
        ustack,
        comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
    };

    // Bump the sample count for this exact stack (small race acceptable for a
    // sampling profiler).
    let next = unsafe { COUNTS.get(&key) }.copied().unwrap_or(0) + 1;
    let _ = COUNTS.insert(&key, &next, 0);
    0
}

#[link_section = "license"]
#[no_mangle]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
