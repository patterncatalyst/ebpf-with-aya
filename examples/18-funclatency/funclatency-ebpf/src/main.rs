//! funclatency-ebpf — time a function by pairing entry and return.
//!
//! uprobe at entry: stash bpf_ktime_get_ns() keyed by pid_tgid.
//! uretprobe at return: delta = now - start; emit one LatEvent.
//!
//! (A production funclatency aggregates a log2 histogram IN the kernel to avoid
//! a per-call event; we emit per-call here so user space can feed a real OTLP
//! histogram. The in-kernel-histogram optimization is noted in the chapter.)
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_ktime_get_ns},
    macros::{map, uprobe, uretprobe},
    maps::{HashMap, RingBuf},
    programs::{ProbeContext, RetProbeContext},
};
use funclatency_common::LatEvent;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[map]
static START: HashMap<u64, u64> = HashMap::with_max_entries(8192, 0);

#[uprobe]
pub fn fn_enter(_ctx: ProbeContext) -> u32 {
    let id = bpf_get_current_pid_tgid();
    let ts = unsafe { bpf_ktime_get_ns() };
    let _ = START.insert(&id, &ts, 0);
    0
}

#[uretprobe]
pub fn fn_exit(_ctx: RetProbeContext) -> u32 {
    let id = bpf_get_current_pid_tgid();
    let start = match unsafe { START.get(&id) } { Some(s) => *s, None => return 0 };
    let _ = START.remove(&id);
    let now = unsafe { bpf_ktime_get_ns() };
    let delta = now.saturating_sub(start);

    if let Some(mut slot) = EVENTS.reserve::<LatEvent>(0) {
        let ev = LatEvent {
            pid: (id >> 32) as u32,
            _pad: 0,
            delta_ns: delta,
            comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
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
