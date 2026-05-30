#![no_std]
#![no_main]
//! Signal program: on sys_enter_execve, if the filename starts with a
//! forbidden prefix, emit a record and bpf_send_signal(SIGKILL) — killing the
//! process before the new image runs. LAB-ONLY: a denylist with lethal force.

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_probe_read_user_str_bytes,
        bpf_send_signal,
    },
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};
use signal_kill_common::KillEvent;

const NEEDLE: &[u8] = b"/tmp/forbidden"; // kill anything exec'd from this prefix
const SIGKILL: u32 = 9;

#[map] static KILLS: RingBuf = RingBuf::with_byte_size(64 * 1024, 0);

#[inline(always)]
fn starts_with(buf: &[u8; 64], needle: &[u8]) -> bool {
    let mut i = 0;
    while i < needle.len() {
        // bounded by the constant needle length → the verifier accepts it
        if buf[i] != needle[i] {
            return false;
        }
        i += 1;
    }
    true
}

#[tracepoint]
pub fn kill_on_exec(ctx: TracePointContext) -> u32 {
    let _ = handle(&ctx);
    0
}

fn handle(ctx: &TracePointContext) -> Result<(), ()> {
    // sys_enter_execve: the filename pointer sits at offset 16 (see Ch 11).
    let fname: *const u8 = unsafe { ctx.read_at(16) }.map_err(|_| ())?;
    let mut buf = [0u8; 64];
    let _ = unsafe { bpf_probe_read_user_str_bytes(fname, &mut buf) };

    if starts_with(&buf, NEEDLE) {
        let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
        if let Some(mut slot) = KILLS.reserve::<KillEvent>(0) {
            let ev = KillEvent { pid, comm: bpf_get_current_comm().unwrap_or_default() };
            unsafe { *slot.as_mut_ptr() = ev; }
            slot.submit(0);
        }
        let _ = unsafe { bpf_send_signal(SIGKILL) };
    }
    Ok(())
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
