//! sslsniff-ebpf — read TLS plaintext via uprobes on OpenSSL's libssl.
//!
//! TLS encrypts the wire, but SSL_write() sees the plaintext BEFORE encryption
//! and SSL_read() sees it AFTER decryption. We attach to libssl.so:
//!   - SSL_write(ssl, buf, num)  uprobe: buf holds plaintext at entry.
//!   - SSL_read(ssl, buf, num)   uprobe: stash buf; uretprobe: read `ret` bytes
//!                               from buf once it's filled.
//!
//! All reads are USER memory (the traced process). Plaintext capture is capped
//! at DATA_CAP bytes per call.
#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_probe_read_user_buf},
    macros::{map, uprobe, uretprobe},
    maps::{HashMap, RingBuf},
    programs::{ProbeContext, RetProbeContext},
};
use aya_log_ebpf::info;
use sslsniff_common::{TlsEvent, DATA_CAP, DIR_READ, DIR_WRITE};

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

/// pid_tgid -> buf pointer captured at SSL_read entry, read back at return.
#[map]
static READ_BUF: HashMap<u64, u64> = HashMap::with_max_entries(4096, 0);

fn emit(dir: u8, buf_ptr: *const u8, len: i64) {
    if len <= 0 {
        return;
    }
    let captured = if (len as usize) < DATA_CAP { len as u32 } else { DATA_CAP as u32 };
    if let Some(mut slot) = EVENTS.reserve::<TlsEvent>(0) {
        let ev = slot.as_mut_ptr();
        unsafe {
            (*ev).pid = (bpf_get_current_pid_tgid() >> 32) as u32;
            (*ev).dir = dir;
            (*ev)._pad = [0; 3];
            (*ev).len = len as u32;
            (*ev).captured = captured;
            (*ev).comm = bpf_get_current_comm().unwrap_or([0u8; 16]);
            (*ev).data = [0u8; DATA_CAP];
            // Copy the captured prefix of the plaintext buffer.
            let _ = bpf_probe_read_user_buf(buf_ptr, &mut (*ev).data[..captured as usize]);
        }
        slot.submit(0);
    }
}

// SSL_write(SSL *ssl, const void *buf, int num): plaintext is at buf on entry.
#[uprobe]
pub fn ssl_write(ctx: ProbeContext) -> u32 {
    let buf: *const u8 = match ctx.arg(1) { Some(p) => p, None => return 0 };
    let num: i64 = ctx.arg(2).unwrap_or(0);
    emit(DIR_WRITE, buf, num);
    info!(&ctx, "SSL_write {} bytes", num);
    0
}

// SSL_read(SSL *ssl, void *buf, int num): buf is filled by the time it returns.
#[uprobe]
pub fn ssl_read_enter(ctx: ProbeContext) -> u32 {
    let buf: u64 = ctx.arg::<*const u8>(1).map(|p| p as u64).unwrap_or(0);
    let id = bpf_get_current_pid_tgid();
    let _ = READ_BUF.insert(&id, &buf, 0);
    0
}

#[uretprobe]
pub fn ssl_read_ret(ctx: RetProbeContext) -> u32 {
    let id = bpf_get_current_pid_tgid();
    let buf = match unsafe { READ_BUF.get(&id) } { Some(b) => *b, None => return 0 };
    let _ = READ_BUF.remove(&id);
    let ret: i64 = ctx.ret().unwrap_or(0);
    emit(DIR_READ, buf as *const u8, ret);
    0
}

#[link_section = "license"]
#[no_mangle]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
