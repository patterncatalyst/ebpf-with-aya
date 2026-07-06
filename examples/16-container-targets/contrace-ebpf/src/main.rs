//! contrace-ebpf — a cgroup-scoped opensnoop.
//!
//! The kernel sees EVERY process regardless of container/namespace. To scope
//! observation to ONE container we filter by its cgroup id: user space writes
//! the target container's cgroup id into TARGET_CGROUP, and we only emit events
//! whose bpf_get_current_cgroup_id() matches. A target of 0 means "all".
//!
//! Traces sys_enter_openat so HTTP requests that open files (e.g. the targets'
//! /work endpoint) produce visible, load-driven, container-scoped events.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_probe_read_user_str_bytes,
    },
    macros::{map, tracepoint},
    maps::{Array, RingBuf},
    programs::TracePointContext,
};
use aya_log_ebpf::info;
use contrace_common::{ContainerEvent, NAME_LEN};

const ENTER_FILENAME_OFF: usize = 24; // const char *filename (see Ch 9)

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

/// One-slot config: the cgroup id to scope to (0 = all). Set from user space.
#[map]
static TARGET_CGROUP: Array<u64> = Array::with_max_entries(1, 0);

#[tracepoint]
pub fn sys_enter_openat(ctx: TracePointContext) -> u32 {
    let _ = try_open(&ctx);
    0
}

fn try_open(ctx: &TracePointContext) -> Result<(), i64> {
    let cgroup = unsafe { bpf_get_current_cgroup_id() };
    let target = TARGET_CGROUP.get(0).copied().unwrap_or(0);
    // Scope: if a target is set, drop everything not in that cgroup.
    if target != 0 && cgroup != target {
        return Ok(());
    }

    if let Some(mut slot) = EVENTS.reserve::<ContainerEvent>(0) {
        let ev = slot.as_mut_ptr();
        unsafe {
            (*ev).pid = (bpf_get_current_pid_tgid() >> 32) as u32;
            (*ev).cgroup = cgroup;
            (*ev).comm = bpf_get_current_comm().unwrap_or([0u8; 16]);
            (*ev).filename = [0u8; NAME_LEN];
            if let Ok(fp) = ctx.read_at::<*const u8>(ENTER_FILENAME_OFF) {
                let _ = bpf_probe_read_user_str_bytes(fp, &mut (*ev).filename);
            }
        }
        slot.submit(0);
    }
    info!(ctx, "open in cgroup {}", cgroup);
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
