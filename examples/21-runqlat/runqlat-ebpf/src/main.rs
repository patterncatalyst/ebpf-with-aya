//! runqlat-ebpf — scheduler run-queue latency, aggregated IN-KERNEL.
//!
//! Run-queue latency = time a task spends RUNNABLE-but-not-running (waiting for
//! a CPU). We time it across the sched tracepoints:
//!   sched_wakeup / sched_wakeup_new : task becomes runnable -> stamp START[pid]
//!   sched_switch : the task coming ON cpu -> delta = now - START[pid] -> HIST
//!                  the task going OFF cpu while still runnable (preempted)
//!                  -> re-stamp START[prev_pid]
//!
//! Context switches are a HOT path, so we DON'T emit per-event (that was Ch 18's
//! note): we increment a log2 histogram in an Array map and let user space read
//! the buckets. Near-zero per-event overhead.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::bpf_ktime_get_ns,
    macros::{map, tracepoint},
    maps::{Array, HashMap},
    programs::TracePointContext,
};
use runqlat_common::NBUCKETS;

// sched_switch field offsets (verify via the format file):
//   prev_pid @ 24, prev_state @ 32 (long), next_pid @ 56
const SW_PREV_PID: usize = 24;
const SW_PREV_STATE: usize = 32;
const SW_NEXT_PID: usize = 56;
// sched_wakeup: pid @ 24
const WAKE_PID: usize = 24;
// TASK_RUNNING == 0 (still runnable -> was preempted)
const TASK_RUNNING: i64 = 0;

#[map]
static START: HashMap<u32, u64> = HashMap::with_max_entries(16384, 0);
#[map]
static HIST: Array<u64> = Array::with_max_entries(NBUCKETS, 0);

fn stamp(pid: u32) {
    if pid == 0 { return; } // ignore the idle task
    let ts = unsafe { bpf_ktime_get_ns() };
    let _ = START.insert(&pid, &ts, 0);
}

fn record(pid: u32) {
    if pid == 0 { return; }
    let start = match unsafe { START.get(&pid) } { Some(s) => *s, None => return };
    let _ = START.remove(&pid);
    let now = unsafe { bpf_ktime_get_ns() };
    let delta_us = now.saturating_sub(start) / 1000;
    let us = delta_us.max(1);
    let bucket = (63 - us.leading_zeros()).min(NBUCKETS - 1); // floor(log2(us))
    if let Some(slot) = HIST.get_ptr_mut(bucket) {
        unsafe { *slot += 1; }
    }
}

#[tracepoint]
pub fn sched_wakeup(ctx: TracePointContext) -> u32 {
    if let Ok(pid) = unsafe { ctx.read_at::<i32>(WAKE_PID) } { stamp(pid as u32); }
    0
}

#[tracepoint]
pub fn sched_wakeup_new(ctx: TracePointContext) -> u32 {
    if let Ok(pid) = unsafe { ctx.read_at::<i32>(WAKE_PID) } { stamp(pid as u32); }
    0
}

#[tracepoint]
pub fn sched_switch(ctx: TracePointContext) -> u32 {
    unsafe {
        // Task leaving the CPU but still runnable -> it's re-queued now.
        if let (Ok(prev_pid), Ok(prev_state)) =
            (ctx.read_at::<i32>(SW_PREV_PID), ctx.read_at::<i64>(SW_PREV_STATE))
        {
            if prev_state == TASK_RUNNING { stamp(prev_pid as u32); }
        }
        // Task coming ON the CPU -> close out its wait.
        if let Ok(next_pid) = ctx.read_at::<i32>(SW_NEXT_PID) {
            record(next_pid as u32);
        }
    }
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
