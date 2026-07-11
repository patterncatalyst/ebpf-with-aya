//! unlinksnoop-ebpf — a kprobe on vfs_unlink().
//!
//! vfs_unlink(struct mnt_idmap *, struct inode *dir, struct dentry *dentry,
//! struct inode **delegated) is the VFS-layer function behind unlink()/
//! unlinkat(). (The older do_unlinkat() helper this chapter first targeted was
//! inlined away on newer kernels — vfs_unlink is the stable target real tools
//! use.) A kprobe fires on its entry, where we read the calling process's
//! identity (stable helpers) and *attempt* to read the unlinked name out of
//! the dentry argument (the version-sensitive part).
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
pub fn vfs_unlink(ctx: ProbeContext) -> u32 {
    match try_vfs_unlink(&ctx) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn try_vfs_unlink(ctx: &ProbeContext) -> Result<(), i64> {
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

    // --- The version-sensitive part: read the unlinked name. ---
    // vfs_unlink's 3rd arg (index 2) is `struct dentry *`. The name lives at
    // dentry->d_name.name. On this kernel the offsets are:
    //   struct dentry.d_name @ 32 (a struct qstr), struct qstr.name @ 8
    // so the `const char *` name pointer sits at dentry + 40. These offsets are
    // kernel-version-specific; if the layout differs the read fails gracefully
    // and the filename stays empty — the event is still emitted with pid/uid/comm.
    const DENTRY_NAME_PTR_OFF: usize = 32 + 8;
    if let Some(dentry) = ctx.arg::<*const u8>(2) {
        unsafe {
            let name_pptr = dentry.add(DENTRY_NAME_PTR_OFF) as *const *const u8;
            if let Ok(p) = bpf_probe_read_kernel::<*const u8>(name_pptr) {
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
