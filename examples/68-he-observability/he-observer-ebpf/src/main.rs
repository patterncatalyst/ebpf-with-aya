//! he-observer-ebpf — time each homomorphic-operation boundary in the workload.
//!
//! One uprobe (`he_enter`) is attached to all four `he_*` symbols; it stamps a
//! start time keyed by pid_tgid. One uretprobe per symbol records the delta and
//! emits a Sample tagged with that operation's id. Because the workload runs its
//! operations sequentially on one thread, a single in-flight start per thread is
//! all we need. Nothing here reads an argument or a return value — only the two
//! timestamps and the (compile-time) operation id. That is the point: the probe
//! cannot see the ciphertext it is timing.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_ktime_get_ns},
    macros::{map, uprobe, uretprobe},
    maps::{HashMap, RingBuf},
    programs::{ProbeContext, RetProbeContext},
};
use he_common::Sample;

#[map]
static START: HashMap<u64, u64> = HashMap::with_max_entries(1024, 0);
#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(64 * 1024, 0);

#[inline(always)]
fn on_entry() {
    let id = bpf_get_current_pid_tgid();
    let now = unsafe { bpf_ktime_get_ns() };
    let _ = START.insert(&id, &now, 0);
}

#[inline(always)]
fn on_return(op: u32) {
    let id = bpf_get_current_pid_tgid();
    if let Some(&start) = unsafe { START.get(&id) } {
        let dur = unsafe { bpf_ktime_get_ns() }.saturating_sub(start);
        if let Some(mut slot) = EVENTS.reserve::<Sample>(0) {
            unsafe { *slot.as_mut_ptr() = Sample { op, _pad: 0, dur_ns: dur }; }
            slot.submit(0);
        }
        let _ = START.remove(&id);
    }
}

#[uprobe]
pub fn he_enter(_ctx: ProbeContext) -> u32 { on_entry(); 0 }

#[uretprobe]
pub fn he_keygen_ret(_ctx: RetProbeContext) -> u32 { on_return(0); 0 }
#[uretprobe]
pub fn he_encrypt_ret(_ctx: RetProbeContext) -> u32 { on_return(1); 0 }
#[uretprobe]
pub fn he_compute_ret(_ctx: RetProbeContext) -> u32 { on_return(2); 0 }
#[uretprobe]
pub fn he_decrypt_ret(_ctx: RetProbeContext) -> u32 { on_return(3); 0 }

#[link_section = "license"]
#[no_mangle]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
