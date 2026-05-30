//! tcpstates-ebpf — trace TCP state transitions via ONE stable tracepoint.
//!
//! sock:inet_sock_set_state fires on every TCP state change and carries the
//! old/new state, addresses, and ports directly — so unlike Ch 27 there are no
//! kprobes and no sock-field offsets to chase. Offsets below are into the
//! tracepoint's format (verify with the format file).
#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};
use tcpstates_common::TcpStateEvent;

// sock:inet_sock_set_state format offsets (verify via the format file):
const OLDSTATE: usize = 16; // int
const NEWSTATE: usize = 20; // int
const SPORT: usize = 24;    // __u16
const DPORT: usize = 26;    // __u16
const PROTOCOL: usize = 30; // __u8
const SADDR: usize = 32;    // __u8[4]
const DADDR: usize = 36;    // __u8[4]
const IPPROTO_TCP: u8 = 6;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[tracepoint]
pub fn inet_sock_set_state(ctx: TracePointContext) -> u32 {
    let _ = try_state(&ctx);
    0
}

fn try_state(ctx: &TracePointContext) -> Result<(), i64> {
    let proto = unsafe { ctx.read_at::<u8>(PROTOCOL) }.map_err(|_| 0i64)?;
    if proto != IPPROTO_TCP {
        return Ok(());
    }
    let ev = TcpStateEvent {
        saddr: unsafe { ctx.read_at::<[u8; 4]>(SADDR) }.unwrap_or([0; 4]),
        daddr: unsafe { ctx.read_at::<[u8; 4]>(DADDR) }.unwrap_or([0; 4]),
        sport: unsafe { ctx.read_at::<u16>(SPORT) }.unwrap_or(0),
        dport: unsafe { ctx.read_at::<u16>(DPORT) }.unwrap_or(0),
        oldstate: unsafe { ctx.read_at::<i32>(OLDSTATE) }.unwrap_or(0) as u32,
        newstate: unsafe { ctx.read_at::<i32>(NEWSTATE) }.unwrap_or(0) as u32,
    };
    if let Some(mut slot) = EVENTS.reserve::<TcpStateEvent>(0) {
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    Ok(())
}

#[link_section = "license"]
#[no_mangle]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
