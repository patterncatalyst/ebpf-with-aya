#![no_std]
#![no_main]
//! XDP UDP load balancer: datagrams to VIP_PORT have their destination port
//! rewritten round-robin across BACKENDS (filled by the loader), then are
//! passed up so the local stack delivers them to the chosen backend.

use core::mem;

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{Array, HashMap},
    programs::XdpContext,
};
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{Ipv4Hdr, IpProto},
    udp::UdpHdr,
};
use xdp_lb_common::VIP_PORT;

#[map] static BACKENDS: Array<u16> = Array::with_max_entries(8, 0); // backend ports
#[map] static NBACK: Array<u32> = Array::with_max_entries(1, 0);    // count of backends
#[map] static IDX: Array<u32> = Array::with_max_entries(1, 0);      // round-robin cursor
#[map] static HITS: HashMap<u16, u64> = HashMap::with_max_entries(8, 0);

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
unsafe fn ptr_at_mut<T>(ctx: &XdpContext, offset: usize) -> Result<*mut T, ()> {
    Ok(ptr_at::<T>(ctx, offset)? as *mut T)
}

#[inline(always)]
fn bump(m: &HashMap<u16, u64>, key: u16, by: u64) {
    let n = unsafe { m.get(&key).copied().unwrap_or(0) } + by;
    let _ = m.insert(&key, &n, 0);
}

#[xdp]
pub fn xdp_lb(ctx: XdpContext) -> u32 {
    try_lb(&ctx).unwrap_or(xdp_action::XDP_PASS)
}

fn try_lb(ctx: &XdpContext) -> Result<u32, ()> {
    let eth: *const EthHdr = unsafe { ptr_at(ctx, 0)? };
    if unsafe { (*eth).ether_type } != EtherType::Ipv4 {
        return Ok(xdp_action::XDP_PASS);
    }
    let ip: *const Ipv4Hdr = unsafe { ptr_at(ctx, EthHdr::LEN)? };
    if unsafe { (*ip).proto } != IpProto::Udp {
        return Ok(xdp_action::XDP_PASS);
    }

    let udp_off = EthHdr::LEN + Ipv4Hdr::LEN;
    let udp: *mut UdpHdr = unsafe { ptr_at_mut(ctx, udp_off)? };
    if unsafe { u16::from_be((*udp).dest) } != VIP_PORT {
        return Ok(xdp_action::XDP_PASS);
    }

    // pick BACKENDS[idx % n]
    let n = unsafe { *NBACK.get(0).ok_or(())? };
    if n == 0 {
        return Ok(xdp_action::XDP_PASS);
    }
    let cur = unsafe { *IDX.get(0).ok_or(())? };
    let port = unsafe { *BACKENDS.get(cur % n).ok_or(())? };
    if let Some(slot) = IDX.get_ptr_mut(0) {
        unsafe { *slot = cur.wrapping_add(1); }
    }

    // rewrite destination port; zero the optional IPv4 UDP checksum
    unsafe {
        (*udp).dest = port.to_be();
        (*udp).check = 0;
    }
    bump(&HITS, port, 1);
    Ok(xdp_action::XDP_PASS)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
