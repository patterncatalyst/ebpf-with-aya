//! hardirqs-ebpf — time spent in hardware-IRQ handlers, per IRQ vector.
//!
//! irq:irq_handler_entry : an IRQ handler starts -> stamp START[cpu]
//! irq:irq_handler_exit  : it finished -> delta = now - START[cpu], add to
//!                         HIST[irq] (count + total_ns)
//!
//! Keyed by CPU because IRQ handlers run per-CPU. Like runqlat this is a hot
//! path, so we AGGREGATE in the kernel (a HashMap of per-IRQ totals) rather
//! than emit per-event. Caveat: nested IRQs on one CPU aren't disentangled.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_smp_processor_id, bpf_ktime_get_ns},
    macros::{map, tracepoint},
    maps::HashMap,
    programs::TracePointContext,
};
use hardirqs_common::IrqStat;

// irq_handler_entry / irq_handler_exit: int irq @ 8 (after the 8-byte common
// header). exit also has int ret @ 12. Verify via the format files.
const IRQ_OFF: usize = 8;

#[map]
static START: HashMap<u32, u64> = HashMap::with_max_entries(1024, 0);
#[map]
static HIST: HashMap<u32, IrqStat> = HashMap::with_max_entries(1024, 0);

#[tracepoint]
pub fn irq_handler_entry(_ctx: TracePointContext) -> u32 {
    let cpu = unsafe { bpf_get_smp_processor_id() };
    let ts = unsafe { bpf_ktime_get_ns() };
    let _ = START.insert(&cpu, &ts, 0);
    0
}

#[tracepoint]
pub fn irq_handler_exit(ctx: TracePointContext) -> u32 {
    let cpu = unsafe { bpf_get_smp_processor_id() };
    let start = match unsafe { START.get(&cpu) } { Some(s) => *s, None => return 0 };
    let _ = START.remove(&cpu);
    let delta = unsafe { bpf_ktime_get_ns() }.saturating_sub(start);

    let irq = match unsafe { ctx.read_at::<i32>(IRQ_OFF) } { Ok(v) => v as u32, Err(_) => return 0 };
    let updated = match unsafe { HIST.get(&irq) } {
        Some(s) => IrqStat { count: s.count + 1, total_ns: s.total_ns + delta },
        None => IrqStat { count: 1, total_ns: delta },
    };
    let _ = HIST.insert(&irq, &updated, 0);
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
