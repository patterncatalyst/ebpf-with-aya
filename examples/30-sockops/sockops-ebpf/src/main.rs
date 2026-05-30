//! sockops-ebpf — a sock_ops program attached to a cgroup.
//!
//! Unlike a tracepoint/kprobe, a sock_ops program is a CALLBACK the TCP stack
//! invokes at socket-lifecycle moments for sockets in the cgroup it's attached
//! to. We react to the two "established" callbacks and emit the connection's
//! direction + 4-tuple — which the context hands us directly (no struct reads).
//!
//! sock_ops can also DO things others can't: set socket options, change
//! congestion control, and enable further callbacks (RTT, state changes) via
//! cb_flags — noted in the chapter.
#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{map, sock_ops},
    maps::RingBuf,
    programs::SockOpsContext,
};
use sockops_common::{SockEvent, DIR_ACTIVE, DIR_PASSIVE};

const BPF_SOCK_OPS_ACTIVE_ESTABLISHED_CB: u32 = 4;
const BPF_SOCK_OPS_PASSIVE_ESTABLISHED_CB: u32 = 5;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[sock_ops]
pub fn track(ctx: SockOpsContext) -> u32 {
    let dir = match ctx.op() {
        BPF_SOCK_OPS_ACTIVE_ESTABLISHED_CB => DIR_ACTIVE,
        BPF_SOCK_OPS_PASSIVE_ESTABLISHED_CB => DIR_PASSIVE,
        _ => return 0,
    };
    if let Some(mut slot) = EVENTS.reserve::<SockEvent>(0) {
        let ev = SockEvent {
            local_ip4: ctx.local_ip4(),
            remote_ip4: ctx.remote_ip4(),
            local_port: ctx.local_port() as u16,
            remote_port: ctx.remote_port() as u16,
            dir,
            _pad: [0; 3],
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
