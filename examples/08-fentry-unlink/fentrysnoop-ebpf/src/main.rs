//! fentrysnoop-ebpf — fentry + fexit on vfs_unlink.
//!
//! (Originally do_unlinkat, which newer kernels inlined away; vfs_unlink is the
//! stable VFS-layer target. It takes 4 args — mnt_idmap, dir, dentry, delegated
//! — so the unlinked name comes from the dentry, and fexit reads the return
//! value at index 4.)
//!
//! fentry/fexit attach to a function's entry/exit using BTF trampolines rather
//! than the int3 breakpoints kprobes use. They're lower overhead, give typed
//! access to arguments via BTF, and — crucially for this program — fexit can
//! read the function's RETURN value, which a single kprobe cannot.
//!
//! Flow: at fentry we capture pid/uid/comm/filename and stash it keyed by
//! pid_tgid in INFLIGHT. At fexit we look it up, attach the return value, emit
//! the completed event to the ring buffer, and clear the entry.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid,
        bpf_probe_read_kernel, bpf_probe_read_kernel_str_bytes,
    },
    macros::{fentry, fexit, map},
    maps::{HashMap, RingBuf},
    programs::{FEntryContext, FExitContext},
};
use aya_log_ebpf::info;
use fentrysnoop_common::{UnlinkEvent, NAME_LEN};

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

/// In-flight unlinks keyed by pid_tgid, bridging entry -> exit.
#[map]
static INFLIGHT: HashMap<u64, UnlinkEvent> = HashMap::with_max_entries(1024, 0);

#[fentry(function = "vfs_unlink")]
pub fn vfs_unlink_enter(ctx: FEntryContext) -> u32 {
    let _ = try_enter(&ctx);
    0
}

fn try_enter(ctx: &FEntryContext) -> Result<(), i64> {
    let id = bpf_get_current_pid_tgid();
    let mut ev = UnlinkEvent {
        pid: (id >> 32) as u32,
        uid: (bpf_get_current_uid_gid() & 0xffff_ffff) as u32,
        ret: 0,
        comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
        filename: [0u8; NAME_LEN],
    };

    // fentry gives typed args; for vfs_unlink arg 2 is `struct dentry *`. The
    // unlinked name is dentry->d_name.name — on this kernel the name pointer is
    // at dentry + 40 (d_name @ 32, qstr.name @ 8). Version-sensitive; reads that
    // miss just leave the filename empty.
    const DENTRY_NAME_PTR_OFF: usize = 32 + 8;
    let dentry = unsafe { ctx.arg::<*const u8>(2) };
    unsafe {
        let name_pptr = dentry.add(DENTRY_NAME_PTR_OFF) as *const *const u8;
        if let Ok(p) = bpf_probe_read_kernel::<*const u8>(name_pptr) {
            let _ = bpf_probe_read_kernel_str_bytes(p, &mut ev.filename);
        }
    }

    INFLIGHT.insert(&id, &ev, 0)?;
    Ok(())
}

#[fexit(function = "vfs_unlink")]
pub fn vfs_unlink_exit(ctx: FExitContext) -> u32 {
    let _ = try_exit(&ctx);
    0
}

fn try_exit(ctx: &FExitContext) -> Result<(), i64> {
    let id = bpf_get_current_pid_tgid();
    let Some(stored) = (unsafe { INFLIGHT.get(&id) }) else {
        return Ok(()); // entry wasn't recorded (map full / raced) — skip
    };
    let mut ev = *stored;

    // In an fexit program the return value follows the function's arguments.
    // vfs_unlink takes 4 args, so the return value is at index 4.
    ev.ret = unsafe { ctx.arg::<i64>(4) } as i32;

    if let Some(mut slot) = EVENTS.reserve::<UnlinkEvent>(0) {
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    info!(ctx, "unlink complete pid {} ret {}", ev.pid, ev.ret);
    let _ = INFLIGHT.remove(&id);
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
