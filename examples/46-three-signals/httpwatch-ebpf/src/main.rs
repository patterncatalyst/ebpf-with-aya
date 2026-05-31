#![no_std]
#![no_main]
//! Language-agnostic request timing at the kernel socket layer. tcp_recvmsg
//! stamps a start keyed by the socket pointer; the following tcp_sendmsg on
//! that socket emits a completed-request event (duration + process comm) to a
//! ring buffer. No symbols, no struct dereferences — works for Java, Python,
//! anything that talks TCP. A deliberate simplification of OBI's L7 parsing.

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_ktime_get_ns},
    macros::{kprobe, map},
    maps::{HashMap, RingBuf},
    programs::ProbeContext,
};
use httpwatch_common::Req;

#[map] static STARTS: HashMap<u64, u64> = HashMap::with_max_entries(10240, 0);
#[map] static EVENTS: RingBuf = RingBuf::with_byte_size(1 << 16, 0);

#[kprobe]
pub fn on_recv(ctx: ProbeContext) -> u32 {
    let sk: u64 = ctx.arg(0).unwrap_or(0); // struct sock *sk
    if sk != 0 {
        let now = unsafe { bpf_ktime_get_ns() };
        let _ = STARTS.insert(&sk, &now, 0);
    }
    0
}

#[kprobe]
pub fn on_send(ctx: ProbeContext) -> u32 {
    let sk: u64 = ctx.arg(0).unwrap_or(0);
    if sk == 0 {
        return 0;
    }
    if let Some(&start) = unsafe { STARTS.get(&sk) } {
        let now = unsafe { bpf_ktime_get_ns() };
        let _ = STARTS.remove(&sk);
        if now <= start {
            return 0;
        }
        if let Some(mut slot) = EVENTS.reserve::<Req>(0) {
            let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);
            unsafe {
                (*slot.as_mut_ptr()).dur_ns = now - start;
                (*slot.as_mut_ptr()).comm = comm;
            }
            slot.submit(0);
        }
    }
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
