//! execsnoop-ebpf — tracepoint on syscalls:sys_enter_execve.
//!
//! New skill vs. Ch 9–10: reading argv, which is `const char *const *` — a USER
//! pointer to an array of USER pointers to strings. We read it in a bounded
//! loop (the verifier requires a constant bound) into fixed per-arg slots so
//! there's no dynamic offset arithmetic to upset the verifier.
//!
//! Offsets from sys_enter_execve/format: filename@16, argv@24, envp@32.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid,
        bpf_probe_read_user, bpf_probe_read_user_str_bytes,
    },
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};
use aya_log_ebpf::info;
use execsnoop_common::{ExecEvent, ARG_LEN, MAX_ARGS, NAME_LEN};

const FILENAME_OFF: usize = 16;
const ARGV_OFF: usize = 24;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

#[tracepoint]
pub fn sys_enter_execve(ctx: TracePointContext) -> u32 {
    let _ = try_execve(&ctx);
    0
}

fn try_execve(ctx: &TracePointContext) -> Result<(), i64> {
    let mut slot = match EVENTS.reserve::<ExecEvent>(0) {
        Some(s) => s,
        None => return Err(0),
    };
    // Write directly into the reserved ring slot (not the 512-byte stack).
    let ev = slot.as_mut_ptr();
    let id = bpf_get_current_pid_tgid();
    unsafe {
        (*ev).pid = (id >> 32) as u32;
        (*ev).uid = (bpf_get_current_uid_gid() & 0xffff_ffff) as u32;
        (*ev).args_count = 0;
        (*ev).comm = bpf_get_current_comm().unwrap_or([0u8; 16]);
        (*ev).filename = [0u8; NAME_LEN];
        (*ev).args = [[0u8; ARG_LEN]; MAX_ARGS];

        // filename (user memory)
        if let Ok(fname) = ctx.read_at::<*const u8>(FILENAME_OFF) {
            let _ = bpf_probe_read_user_str_bytes(fname, &mut (*ev).filename);
        }

        // argv: user pointer to an array of user string pointers
        if let Ok(argv) = ctx.read_at::<*const *const u8>(ARGV_OFF) {
            let mut count = 0u32;
            for i in 0..MAX_ARGS {
                // read the i-th pointer out of the user array
                let argp = match bpf_probe_read_user::<*const u8>(argv.add(i)) {
                    Ok(p) => p,
                    Err(_) => break,
                };
                if argp.is_null() {
                    break;
                }
                let _ = bpf_probe_read_user_str_bytes(argp, &mut (*ev).args[i]);
                count += 1;
            }
            (*ev).args_count = count;
        }
    }
    info!(ctx, "execve pid {}", unsafe { (*ev).pid });
    slot.submit(0);
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
