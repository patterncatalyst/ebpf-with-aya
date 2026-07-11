//! httpl7-ebpf — an L7 (HTTP) socket filter.
//!
//! A socket_filter program runs on every packet delivered to the socket it's
//! attached to. We walk Ethernet → IPv4 → TCP to find the payload, and if it
//! starts with an HTTP method (or "HTTP/" for responses) we capture the first
//! line and emit it with the 4-tuple.
//!
//! Simplifying assumptions (verifier-friendly, flagged in the chapter):
//!   - IPv4 with NO options (IHL == 5, so IP header is 20 bytes)
//!   - parse the TCP data offset to skip TCP options
#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::__sk_buff,
    helpers::bpf_skb_load_bytes,
    macros::{map, socket_filter},
    maps::RingBuf,
    programs::SkBuffContext,
    EbpfContext,
};
use httpl7_common::{HttpEvent, LINE_CAP};

const ETH_HLEN: usize = 14;
const ETH_P_IP: u16 = 0x0800;
const IPPROTO_TCP: u8 = 6;

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

fn looks_http(b: &[u8; 5]) -> bool {
    matches!(b,
        [b'G', b'E', b'T', b' ', _] |
        [b'P', b'O', b'S', b'T', _] |
        [b'P', b'U', b'T', b' ', _] |
        [b'H', b'E', b'A', b'D', _] |
        [b'D', b'E', b'L', b'E', _] |
        [b'H', b'T', b'T', b'P', b'/'])
}

#[socket_filter]
pub fn http_filter(ctx: SkBuffContext) -> i64 {
    match try_http(&ctx) {
        Ok(_) => 0,
        Err(_) => 0,
    }
}

fn try_http(ctx: &SkBuffContext) -> Result<(), i64> {
    // Ethertype must be IPv4.
    let ethertype = u16::from_be(ctx.load::<u16>(12).map_err(|_| 1i64)?);
    if ethertype != ETH_P_IP { return Ok(()); }

    // IPv4, no options only.
    let verihl = ctx.load::<u8>(ETH_HLEN).map_err(|_| 1i64)?;
    if (verihl & 0x0f) != 5 { return Ok(()); }
    let proto = ctx.load::<u8>(ETH_HLEN + 9).map_err(|_| 1i64)?;
    if proto != IPPROTO_TCP { return Ok(()); }

    let saddr = ctx.load::<u32>(ETH_HLEN + 12).map_err(|_| 1i64)?;
    let daddr = ctx.load::<u32>(ETH_HLEN + 16).map_err(|_| 1i64)?;
    let tcp_off = ETH_HLEN + 20;
    let sport = ctx.load::<u16>(tcp_off).map_err(|_| 1i64)?;
    let dport = ctx.load::<u16>(tcp_off + 2).map_err(|_| 1i64)?;

    // TCP data offset (high nibble of byte 12) → header length in bytes.
    let doff = ctx.load::<u8>(tcp_off + 12).map_err(|_| 1i64)?;
    let tcp_hlen = ((doff >> 4) as usize) * 4;
    let payload = tcp_off + tcp_hlen;

    let first5: [u8; 5] = ctx.load::<[u8; 5]>(payload).map_err(|_| 1i64)?;
    if !looks_http(&first5) { return Ok(()); }

    let mut line = [0u8; LINE_CAP];
    // Bounded copy of the first line (best effort; trailing bytes may be \r\n…).
    // aya's load_bytes computes len = min(skb.len - offset, LINE_CAP), which can
    // be 0 — and bpf_skb_load_bytes rejects a zero-length read on modern kernels
    // ("invalid zero-sized read"). Read directly with a provably-nonzero length.
    let skb = ctx.as_ptr() as *mut __sk_buff;
    let avail = (unsafe { (*skb).len } as usize).saturating_sub(payload);
    // Clamp to 1..=LINE_CAP: .max(1) makes it provably nonzero (no zero-sized
    // read) and .min(LINE_CAP) keeps the write inside `line`. If there's no
    // payload the 1-byte read just fails and `line` stays zeroed.
    let n = avail.min(LINE_CAP).max(1);
    unsafe {
        let _ = bpf_skb_load_bytes(
            skb as *const _,
            payload as u32,
            line.as_mut_ptr() as *mut _,
            n as u32,
        );
    }

    if let Some(mut slot) = EVENTS.reserve::<HttpEvent>(0) {
        let ev = HttpEvent { saddr, daddr, sport, dport, len: LINE_CAP as u32, line };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
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
