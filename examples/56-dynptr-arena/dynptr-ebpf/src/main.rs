#![no_std]
#![no_main]
//! Aya rendering of the dynptr producer. aya's ring-buffer *dynptr* reserve is
//! emerging, so this reserves a fixed-size Record and records the logical
//! `len`; reference/dynptr_ringbuf.bpf.c shows the true variable-length dynptr
//! reservation. Either way the loader reads `len` bytes per record.

use aya_ebpf::{
    helpers::bpf_get_current_pid_tgid,
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};
use dynptr_common::Record;

#[map] static RB: RingBuf = RingBuf::with_byte_size(1 << 16, 0);

#[tracepoint]
pub fn emit(_ctx: TracePointContext) -> u32 {
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    // logical, runtime-varying length: short for even pids, long for odd
    let len = if pid & 1 == 0 { 16u32 } else { 56u32 };
    if let Some(mut slot) = RB.reserve::<Record>(0) {
        let r = unsafe { &mut *slot.as_mut_ptr() };
        r.pid = pid;
        r.len = len;
        slot.submit(0);
    }
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
