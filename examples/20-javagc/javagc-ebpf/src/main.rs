//! javagc-ebpf — time JVM GC pauses by uprobing the collector's pause function.
//!
//! The "textbook" way to do this is the HotSpot USDT probes
//! ("hotspot:gc__begin" / "hotspot:gc__end"). But those only exist in an
//! OpenJDK built with `--enable-dtrace`, which Fedora's (and most Linux distros')
//! OpenJDK is NOT — so on a stock JDK there are no gc USDT markers to attach to.
//!
//! Instead we uprobe the collector's real stop-the-world entry point,
//! `G1CollectedHeap::do_collection_pause_at_safepoint`, resolved by symbol name
//! from libjvm.so's (unstripped) `.symtab`. A `#[uprobe]` on entry records the
//! start timestamp; a `#[uretprobe]` on the same function's return computes the
//! pause. This catches *every* automatic G1 pause (young/mixed/full), not just
//! explicit `System.gc()`. Keyed by tid — the VM thread runs the pause, so the
//! entry and the matching return share the same kernel tid.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_ktime_get_ns},
    macros::{map, uprobe, uretprobe},
    maps::{HashMap, RingBuf},
    programs::{ProbeContext, RetProbeContext},
};
use aya_log_ebpf::info;
use javagc_common::GcEvent;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);
#[map]
static GC_START: HashMap<u32, u64> = HashMap::with_max_entries(1024, 0); // key = tid

#[uprobe]
pub fn gc_begin(_ctx: ProbeContext) -> u32 {
    let tid = bpf_get_current_pid_tgid() as u32; // low 32 bits = kernel tid
    let ts = unsafe { bpf_ktime_get_ns() };
    let _ = GC_START.insert(&tid, &ts, 0);
    0
}

#[uretprobe]
pub fn gc_end(ctx: RetProbeContext) -> u32 {
    let id = bpf_get_current_pid_tgid();
    let tid = id as u32;
    let pid = (id >> 32) as u32;
    let start = match unsafe { GC_START.get(&tid) } { Some(s) => *s, None => return 0 };
    let _ = GC_START.remove(&tid);
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
