#![no_std]
#![no_main]
//! Probe postgres: per-query latency + SQL text (exec_simple_query) and
//! lock-wait time (ProcSleep), keyed by the single-threaded backend pid. One
//! uprobe on the postgres binary covers every backend process at once.

use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_ktime_get_ns, bpf_probe_read_user_str_bytes},
    macros::{map, uprobe, uretprobe},
    maps::{HashMap, RingBuf},
    programs::{ProbeContext, RetProbeContext},
};
use pg_probe_common::{Event, KIND_LOCK, KIND_QUERY};

#[map] static QSTART: HashMap<u64, u64> = HashMap::with_max_entries(4096, 0);
#[map] static QTEXT: HashMap<u64, [u8; 128]> = HashMap::with_max_entries(4096, 0);
#[map] static LSTART: HashMap<u64, u64> = HashMap::with_max_entries(4096, 0);
#[map] static EVENTS: RingBuf = RingBuf::with_byte_size(1 << 16, 0);

#[inline(always)]
fn emit(kind: u32, key: u64, dur: u64, text: Option<[u8; 128]>) {
    if let Some(mut slot) = EVENTS.reserve::<Event>(0) {
        let e = unsafe { &mut *slot.as_mut_ptr() };
        e.kind = kind;
        e.pid = (key >> 32) as u32;
        e.dur_ns = dur;
        e.query = text.unwrap_or([0u8; 128]);
        slot.submit(0);
    }
}

#[uprobe] // exec_simple_query(const char *query_string)
pub fn q_start(ctx: ProbeContext) -> u32 {
    let key = bpf_get_current_pid_tgid();
    let qptr: u64 = ctx.arg(0).unwrap_or(0);
    if qptr != 0 {
        let mut buf = [0u8; 128];
        let _ = unsafe { bpf_probe_read_user_str_bytes(qptr as *const u8, &mut buf) };
        let _ = QTEXT.insert(&key, &buf, 0);
    }
    let _ = QSTART.insert(&key, &unsafe { bpf_ktime_get_ns() }, 0);
    0
}

#[uretprobe]
pub fn q_done(_ctx: RetProbeContext) -> u32 {
    let key = bpf_get_current_pid_tgid();
    if let Some(&start) = unsafe { QSTART.get(&key) } {
        let now = unsafe { bpf_ktime_get_ns() };
        let text = unsafe { QTEXT.get(&key).copied() };
        if now > start {
            emit(KIND_QUERY, key, now - start, text);
        }
        let _ = QSTART.remove(&key);
        let _ = QTEXT.remove(&key);
    }
    0
}

#[uprobe] // ProcSleep(...) — a backend only calls this while blocked on a lock
pub fn l_start(_ctx: ProbeContext) -> u32 {
    let key = bpf_get_current_pid_tgid();
    let _ = LSTART.insert(&key, &unsafe { bpf_ktime_get_ns() }, 0);
    0
}

#[uretprobe]
pub fn l_done(_ctx: RetProbeContext) -> u32 {
    let key = bpf_get_current_pid_tgid();
    if let Some(&start) = unsafe { LSTART.get(&key) } {
        let now = unsafe { bpf_ktime_get_ns() };
        if now > start {
            emit(KIND_LOCK, key, now - start, None);
        }
        let _ = LSTART.remove(&key);
    }
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
