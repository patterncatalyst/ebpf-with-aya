#![no_std]
#![no_main]
//! XDP capture: parse Ethernet/IPv4/TCP, count every packet by protocol in
//! SEEN, and for TCP control packets (SYN/FIN/RST) ship a small FlowRecord to
//! user space via a RingBuf. Always returns XDP_PASS — a read-only tap.

use core::mem;

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{HashMap, RingBuf},
    programs::XdpContext,
};
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{Ipv4Hdr, IpProto},
    tcp::TcpHdr,
};
use xdp_capture_common::{FlowRecord, TCP_FIN, TCP_RST, TCP_SYN};

#[map] static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);
#[map] static SEEN: HashMap<u32, u64> = HashMap::with_max_entries(4, 0);

#[inline(always)]
unsafe fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    if start + offset + mem::size_of::<T>() > end {
        return Err(());
    }
    Ok((start + offset) as *const T)
}

#[inline(always)]
fn bump(m: &HashMap<u32, u64>, key: u32, by: u64) {
    let n = unsafe { m.get(&key).copied().unwrap_or(0) } + by;
    let _ = m.insert(&key, &n, 0);
}

#[xdp]
pub fn xdp_capture(ctx: XdpContext) -> u32 {
    try_capture(&ctx).unwrap_or(xdp_action::XDP_PASS)
}

fn try_capture(ctx: &XdpContext) -> Result<u32, ()> {
    let eth: *const EthHdr = unsafe { ptr_at(ctx, 0)? };
    if unsafe { (*eth).ether_type } != EtherType::Ipv4 {
        return Ok(xdp_action::XDP_PASS);
    }
    let ip: *const Ipv4Hdr = unsafe { ptr_at(ctx, EthHdr::LEN)? };
    let proto = unsafe { (*ip).proto };
    bump(&SEEN, proto as u32, 1);
    if proto != IpProto::Tcp {
        return Ok(xdp_action::XDP_PASS);
    }

    let tcp_off = EthHdr::LEN + Ipv4Hdr::LEN;
    let tcp: *const TcpHdr = unsafe { ptr_at(ctx, tcp_off)? };
    let flags: u8 = unsafe { *ptr_at::<u8>(ctx, tcp_off + 13)? }; // TCP flags byte

    if flags & (TCP_SYN | TCP_FIN | TCP_RST) != 0 {
        if let Some(mut slot) = EVENTS.reserve::<FlowRecord>(0) {
            let rec = FlowRecord {
                saddr: unsafe { (*ip).src_addr },
                daddr: unsafe { (*ip).dst_addr },
                sport: unsafe { u16::from_be((*tcp).source) },
                dport: unsafe { u16::from_be((*tcp).dest) },
                flags,
                len: unsafe { u16::from_be((*ip).tot_len) },
            };
            unsafe { *slot.as_mut_ptr() = rec; }
            slot.submit(0);
        }
    }
    Ok(xdp_action::XDP_PASS)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
