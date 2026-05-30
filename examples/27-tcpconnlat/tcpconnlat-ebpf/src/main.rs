//! tcpconnlat-ebpf — active TCP connection latency (connect → SYN-ACK).
//!
//! kprobe tcp_v4_connect(sk): stamp t0, keyed by the struct sock* pointer, and
//!   grab the destination (addr/port live at the head of struct sock_common).
//! kprobe tcp_rcv_state_process(sk, skb): the first time we see our sk here is
//!   the SYN-ACK being processed — compute the latency, emit, and forget the sk.
//!
//! Reading sock fields by fixed offset is the fragile part: the head of
//! struct sock (sock_common) is fairly stable, but production code uses CO-RE
//! to relocate these — that's Chapter 56. Offsets are flagged to verify.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_ktime_get_ns, bpf_probe_read_kernel},
    macros::{kprobe, map},
    maps::{HashMap, RingBuf},
    programs::ProbeContext,
};
use tcpconnlat_common::{ConnEvent, ConnStart};

// Head of struct sock_common (verify with: pahole -C sock_common):
//   skc_daddr @ 0 (__be32), skc_dport @ 12 (__be16)
const SKC_DADDR: usize = 0;
const SKC_DPORT: usize = 12;

#[map]
static START: HashMap<u64, ConnStart> = HashMap::with_max_entries(16384, 0);
#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[kprobe]
pub fn tcp_v4_connect(ctx: ProbeContext) -> u32 {
    let sk: u64 = match ctx.arg(0) { Some(p) => p, None => return 0 };
    let daddr = unsafe { bpf_probe_read_kernel((sk as usize + SKC_DADDR) as *const u32) }.unwrap_or(0);
    let dport = unsafe { bpf_probe_read_kernel((sk as usize + SKC_DPORT) as *const u16) }.unwrap_or(0);
    let start = ConnStart {
        ts: unsafe { bpf_ktime_get_ns() },
        pid: (bpf_get_current_pid_tgid() >> 32) as u32,
        daddr, dport, _pad: 0,
        comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
    };
    let _ = START.insert(&sk, &start, 0);
    0
}

#[kprobe]
pub fn tcp_rcv_state_process(ctx: ProbeContext) -> u32 {
    let sk: u64 = match ctx.arg(0) { Some(p) => p, None => return 0 };
    let start = match unsafe { START.get(&sk) } { Some(s) => *s, None => return 0 };
    let _ = START.remove(&sk); // first hit ≈ SYN-ACK; don't re-fire
    let lat = unsafe { bpf_ktime_get_ns() }.saturating_sub(start.ts);

    if let Some(mut slot) = EVENTS.reserve::<ConnEvent>(0) {
        let ev = ConnEvent {
            pid: start.pid, daddr: start.daddr, dport: start.dport, _pad: 0,
            lat_ns: lat, comm: start.comm,
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
