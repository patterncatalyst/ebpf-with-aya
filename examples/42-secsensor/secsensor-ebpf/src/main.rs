#![no_std]
#![no_main]
//! A small security sensor: three attacker-relevant tracepoints (execve,
//! ptrace, setuid) emit one uniform SecEvent stream over a single RingBuf.
//! Observe-only — the user side classifies and scores; shielding is an LSM
//! add-on (see the chapter).

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid},
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};
use secsensor_common::{SecEvent, ET_EXEC, ET_PTRACE, ET_SETUID};

#[map] static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[inline(always)]
fn emit(etype: u32) {
    if let Some(mut slot) = EVENTS.reserve::<SecEvent>(0) {
        let ev = SecEvent {
            etype,
            pid: (bpf_get_current_pid_tgid() >> 32) as u32,
            comm: bpf_get_current_comm().unwrap_or_default(),
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
}

#[tracepoint]
pub fn on_exec(_ctx: TracePointContext) -> u32 {
    emit(ET_EXEC);
    0
}

#[tracepoint]
pub fn on_ptrace(_ctx: TracePointContext) -> u32 {
    emit(ET_PTRACE);
    0
}

#[tracepoint]
pub fn on_setuid(_ctx: TracePointContext) -> u32 {
    emit(ET_SETUID);
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
