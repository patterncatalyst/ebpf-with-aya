//! energy-ebpf — accumulate per-task on-CPU time, the basis for energy
//! attribution.
//!
//! On every sched_switch: the task leaving the CPU (prev) gets credited the
//! time it just spent running; the task arriving (next) starts its clock. User
//! space turns each task's CPU-time SHARE into a power estimate using RAPL (or
//! a model when RAPL isn't exposed) — the same utilization model Kepler uses in
//! clouds where hardware energy counters aren't available.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_smp_processor_id, bpf_ktime_get_ns},
    macros::{map, tracepoint},
    maps::HashMap,
    programs::TracePointContext,
};
use energy_common::{TaskStat, COMM_LEN};

// sched_switch offsets (verify via the format file): prev_comm[16] @ 8,
// prev_pid @ 24, next_pid @ 56.
const PREV_COMM: usize = 8;
const PREV_PID: usize = 24;
const NEXT_PID: usize = 56;

#[map]
static ONCPU: HashMap<u32, u64> = HashMap::with_max_entries(1024, 0); // cpu -> ts
#[map]
static USAGE: HashMap<u32, TaskStat> = HashMap::with_max_entries(16384, 0); // pid -> stat

#[tracepoint]
pub fn sched_switch(ctx: TracePointContext) -> u32 {
    let cpu = unsafe { bpf_get_smp_processor_id() };
    let now = unsafe { bpf_ktime_get_ns() };

    // Credit the outgoing task for the slice it just ran.
    if let Some(&start) = unsafe { ONCPU.get(&cpu) } {
        let delta = now.saturating_sub(start);
        if let Ok(prev_pid) = unsafe { ctx.read_at::<i32>(PREV_PID) } {
            let prev_pid = prev_pid as u32;
            if prev_pid != 0 {
                let comm = unsafe { ctx.read_at::<[u8; COMM_LEN]>(PREV_COMM) }.unwrap_or([0u8; COMM_LEN]);
                let cur = unsafe { USAGE.get(&prev_pid) }.copied()
                    .unwrap_or(TaskStat { cpu_ns: 0, comm });
                let upd = TaskStat { cpu_ns: cur.cpu_ns + delta, comm };
                let _ = USAGE.insert(&prev_pid, &upd, 0);
            }
        }
    }

    // Start the incoming task's clock on this CPU.
    let _ = ONCPU.insert(&cpu, &now, 0);
    let _ = ctx.read_at::<i32>(NEXT_PID); // (read kept for symmetry/clarity)
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
