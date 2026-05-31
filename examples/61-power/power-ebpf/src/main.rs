#![no_std]
#![no_main]
//! On every sched_switch, charge the outgoing task's on-CPU slice to its command
//! name. These per-workload time shares are the weights the loader multiplies by
//! RAPL package energy to estimate per-workload power.

use aya_ebpf::{
    helpers::bpf_ktime_get_ns,
    macros::{map, tracepoint},
    maps::{HashMap, PerCpuArray},
    programs::TracePointContext,
};
use power_common::Comm;

// per-command cumulative on-CPU nanoseconds
#[map] static ONCPU: HashMap<Comm, u64> = HashMap::with_max_entries(4096, 0);
// per-CPU timestamp of the last switch (when the current task was scheduled in)
#[map] static LAST: PerCpuArray<u64> = PerCpuArray::with_max_entries(1, 0);

// sched_switch layout: prev_comm @ 8 (char[16]), prev_pid @ 24
const PREV_COMM_OFF: usize = 8;

#[tracepoint]
pub fn on_switch(ctx: TracePointContext) -> u32 {
    let _ = try_switch(&ctx);
    0
}

fn try_switch(ctx: &TracePointContext) -> Result<(), i64> {
    let now = unsafe { bpf_ktime_get_ns() };
    if let Some(last) = LAST.get_ptr_mut(0) {
        let prev_ts = unsafe { *last };
        if prev_ts != 0 {
            let delta = now.saturating_sub(prev_ts);
            let name: [u8; 16] = unsafe { ctx.read_at(PREV_COMM_OFF)? };
            let key = Comm { name };
            let cur = unsafe { ONCPU.get(&key) }.copied().unwrap_or(0);
            let _ = ONCPU.insert(&key, &(cur + delta), 0);
        }
        unsafe { *last = now };
    }
    Ok(())
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
