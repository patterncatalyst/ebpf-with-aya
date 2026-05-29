//! javagc-ebpf — time JVM GC pauses via the HotSpot USDT probes.
//!
//! A USDT probe ("hotspot:gc__begin" / "hotspot:gc__end") is just a marker at a
//! fixed instruction offset in libjvm.so. We attach a plain uprobe at each
//! offset (resolved in user space from the ELF .note.stapsdt section) and time
//! begin -> end with the entry/exit pattern.
//!
//! Two uprobes, no uretprobes — begin and end are SEPARATE probe sites, so no
//! return-trampoline issues. Keyed by pid (GC is stop-the-world per JVM).
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_ktime_get_ns},
    macros::{map, uprobe},
    maps::{HashMap, RingBuf},
    programs::ProbeContext,
};
use aya_log_ebpf::info;
use javagc_common::GcEvent;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);
#[map]
static GC_START: HashMap<u32, u64> = HashMap::with_max_entries(1024, 0);

#[uprobe]
pub fn gc_begin(_ctx: ProbeContext) -> u32 {
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let ts = unsafe { bpf_ktime_get_ns() };
    let _ = GC_START.insert(&pid, &ts, 0);
    0
}

#[uprobe]
pub fn gc_end(ctx: ProbeContext) -> u32 {
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let start = match unsafe { GC_START.get(&pid) } { Some(s) => *s, None => return 0 };
    let _ = GC_START.remove(&pid);
    let pause = unsafe { bpf_ktime_get_ns() }.saturating_sub(start);

    if let Some(mut slot) = EVENTS.reserve::<GcEvent>(0) {
        let ev = GcEvent {
            pid,
            _pad: 0,
            pause_ns: pause,
            comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    info!(&ctx, "GC pause {} ns", pause);
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
