//! unlinksnoop-ebpf — a kprobe on do_unlinkat().
//!
//! do_unlinkat(int dfd, struct filename *name) is the kernel function behind
//! the unlink()/unlinkat() syscalls. A kprobe fires on its entry, where we can
//! read the calling process's identity (stable helpers) and *attempt* to read
//! the filename out of the second argument (the version-sensitive part).
//!
//! Each hit pushes an UnlinkEvent into a ring buffer for user space to drain.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid, bpf_probe_read_kernel_str_bytes},
    macros::{kprobe, map},
    maps::RingBuf,
    programs::ProbeContext,
};
use aya_log_ebpf::info;
use unlinksnoop_common::{UnlinkEvent, NAME_LEN};

/// Ring buffer sized 256 KiB (power of two, as the kernel requires).
#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[kprobe]
pub fn do_unlinkat(ctx: ProbeContext) -> u32 {
    match try_do_unlinkat(&ctx) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn try_do_unlinkat(ctx: &ProbeContext) -> Result<(), i64> {
    // Reserve a slot in the ring buffer for one event.
    let mut entry = match EVENTS.reserve::<UnlinkEvent>(0) {
        Some(e) => e,
        None => return Err(0),
    };
    let event = entry.as_mut_ptr();

    // Stable, always-available process context.
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let uid = (bpf_get_current_uid_gid() & 0xffff_ffff) as u32;
    unsafe {
        (*event).pid = pid;
        (*event).uid = uid;
        (*event).comm = bpf_get_current_comm().unwrap_or([0u8; 16]);
        (*event).filename = [0u8; NAME_LEN];
    }

    // --- The version-sensitive part: read the filename. ---
    // do_unlinkat's 2nd arg is `struct filename *`. The path string lives at
    // `filename->name` (a `const char *`). We read the pointer, then the bytes.
    // If the layout differs on your kernel, this read fails gracefully and the
    // filename stays empty — the event is still emitted with pid/uid/comm.
    if let Some(name_ptr) = ctx.arg::<*const u8>(1) {
        unsafe {
            // `struct filename` begins with `const char *name;` on current
            // kernels, so the first pointer-sized field is the path pointer.
            let path_ptr = bpf_probe_read_kernel::<*const u8>(name_ptr as *const *const u8);
            if let Ok(p) = path_ptr {
                let dst = &mut (*event).filename;
                let _ = bpf_probe_read_kernel_str_bytes(p, dst);
            }
        }
    }

    info!(ctx, "unlink by pid {} uid {}", pid, uid);
    entry.submit(0);
    Ok(())
}

use aya_ebpf::helpers::bpf_probe_read_kernel;

#[link_section = "license"]
#[no_mangle]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
