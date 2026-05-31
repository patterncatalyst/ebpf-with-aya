#![no_std]
#![no_main]
//! An Aya probe to *observe* a running scheduler (sched_ext or the default):
//! count context switches per CPU via the sched:sched_switch tracepoint.

use aya_ebpf::{
    helpers::bpf_get_smp_processor_id,
    macros::{map, tracepoint},
    maps::HashMap,
    programs::TracePointContext,
};

#[map] static CTXSW: HashMap<u32, u64> = HashMap::with_max_entries(256, 0);

#[tracepoint]
pub fn on_switch(_ctx: TracePointContext) -> u32 {
    let cpu = unsafe { bpf_get_smp_processor_id() };
    let n = unsafe { CTXSW.get(&cpu).copied().unwrap_or(0) } + 1;
    let _ = CTXSW.insert(&cpu, &n, 0);
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
