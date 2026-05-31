#![no_std]
#![no_main]
//! Per-CPU busy-time probe: on each sched_switch, attribute the interval since
//! the last switch on that CPU to "busy" if the outgoing task was not the idle
//! task (prev_pid != 0). Reveals how a scheduler concentrates work.

use aya_ebpf::{
    helpers::{bpf_get_smp_processor_id, bpf_ktime_get_ns},
    macros::{map, tracepoint},
    maps::HashMap,
    programs::TracePointContext,
};

#[map] static LAST: HashMap<u32, u64> = HashMap::with_max_entries(256, 0); // cpu -> last switch ts
#[map] static BUSY: HashMap<u32, u64> = HashMap::with_max_entries(256, 0); // cpu -> accumulated busy ns

#[tracepoint]
pub fn on_switch(ctx: TracePointContext) -> u32 {
    let cpu = unsafe { bpf_get_smp_processor_id() };
    let now = unsafe { bpf_ktime_get_ns() };
    // sched:sched_switch — prev_pid is at offset 24 (after the common header
    // and prev_comm[16]); 0 means the idle task (swapper).
    let prev_pid: i32 = unsafe { ctx.read_at(24) }.unwrap_or(0);

    if let Some(&last) = unsafe { LAST.get(&cpu) } {
        if prev_pid != 0 && now > last {
            let n = unsafe { BUSY.get(&cpu).copied().unwrap_or(0) } + (now - last);
            let _ = BUSY.insert(&cpu, &n, 0);
        }
    }
    let _ = LAST.insert(&cpu, &now, 0);
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
