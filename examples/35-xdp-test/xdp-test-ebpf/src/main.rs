#![no_std]
#![no_main]
//! The program under test: drop ICMP, pass everything else, and count packets
//! per protocol in PKTS. Exercised by the user-space harness via
//! BPF_PROG_TEST_RUN — never actually attached to an interface here.

use core::mem;

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::HashMap,
    programs::XdpContext,
};
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{Ipv4Hdr, IpProto},
};

#[map] static PKTS: HashMap<u32, u64> = HashMap::with_max_entries(8, 0);

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
fn bump(m: &HashMap<u32, u64>, k: u32, by: u64) {
    let n = unsafe { m.get(&k).copied().unwrap_or(0) } + by;
    let _ = m.insert(&k, &n, 0);
}

#[xdp]
pub fn xdp_filter(ctx: XdpContext) -> u32 {
    try_filter(&ctx).unwrap_or(xdp_action::XDP_PASS)
}

fn try_filter(ctx: &XdpContext) -> Result<u32, ()> {
    let eth: *const EthHdr = unsafe { ptr_at(ctx, 0)? };
    if unsafe { (*eth).ether_type } != EtherType::Ipv4 {
        return Ok(xdp_action::XDP_PASS);
    }
    let ip: *const Ipv4Hdr = unsafe { ptr_at(ctx, EthHdr::LEN)? };
    let proto = unsafe { (*ip).proto };
    bump(&PKTS, proto as u32, 1);
    if proto == IpProto::Icmp {
        return Ok(xdp_action::XDP_DROP);
    }
    Ok(xdp_action::XDP_PASS)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
