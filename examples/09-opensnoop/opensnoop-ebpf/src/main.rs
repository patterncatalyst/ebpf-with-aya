//! opensnoop-ebpf — tracepoints on the openat syscall.
//!
//! Unlike Chapters 7–8 (which attached to a kernel *function*), this attaches
//! to stable *tracepoints*: syscalls:sys_enter_openat (args: filename, flags)
//! and syscalls:sys_exit_openat (the return value = fd or -errno). Tracepoint
//! arguments are read from the context at fixed offsets taken from the event's
//! format file:
//!   cat /sys/kernel/tracing/events/syscalls/sys_enter_openat/format
//!
//! Note: the filename pointer at syscall entry points to USER memory, so we use
//! bpf_probe_read_USER_str_bytes — contrast with the kernel reads in Ch 7–8.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid,
        bpf_probe_read_user_str_bytes,
    },
    macros::{map, tracepoint},
    maps::{HashMap, RingBuf},
    programs::TracePointContext,
};
use aya_log_ebpf::info;
use opensnoop_common::{OpenEvent, NAME_LEN};

// Offsets within the tracepoint record (from the format file). Verify on your
// kernel; these are the long-stable x86_64 values.
const ENTER_FILENAME_OFF: usize = 24; // const char *filename
const ENTER_FLAGS_OFF: usize = 32; // int flags
const EXIT_RET_OFF: usize = 16; // long ret

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);
#[map]
static INFLIGHT: HashMap<u64, OpenEvent> = HashMap::with_max_entries(4096, 0);

#[tracepoint]
pub fn sys_enter_openat(ctx: TracePointContext) -> u32 {
    let _ = try_enter(&ctx);
    0
}

fn try_enter(ctx: &TracePointContext) -> Result<(), i64> {
    let id = bpf_get_current_pid_tgid();
    let mut ev = OpenEvent {
        pid: (id >> 32) as u32,
        uid: (bpf_get_current_uid_gid() & 0xffff_ffff) as u32,
        ret: 0,
        flags: 0,
        comm: bpf_get_current_comm().unwrap_or([0u8; 16]),
        filename: [0u8; NAME_LEN],
    };
    unsafe {
        let filename_ptr: *const u8 = ctx.read_at(ENTER_FILENAME_OFF)?;
        ev.flags = ctx.read_at::<i32>(ENTER_FLAGS_OFF)?;
        // filename is a USER pointer at syscall entry.
        let _ = bpf_probe_read_user_str_bytes(filename_ptr, &mut ev.filename);
    }
    INFLIGHT.insert(&id, &ev, 0)?;
    Ok(())
}

#[tracepoint]
pub fn sys_exit_openat(ctx: TracePointContext) -> u32 {
    let _ = try_exit(&ctx);
    0
}

fn try_exit(ctx: &TracePointContext) -> Result<(), i64> {
    let id = bpf_get_current_pid_tgid();
    let Some(stored) = (unsafe { INFLIGHT.get(&id) }) else {
        return Ok(());
    };
    let mut ev = *stored;
    ev.ret = unsafe { ctx.read_at::<i64>(EXIT_RET_OFF)? } as i32;

    if let Some(mut slot) = EVENTS.reserve::<OpenEvent>(0) {
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    info!(ctx, "openat pid {} ret {}", ev.pid, ev.ret);
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
