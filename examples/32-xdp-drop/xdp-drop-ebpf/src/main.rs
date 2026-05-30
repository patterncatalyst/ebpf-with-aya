#![no_std]
#![no_main]
//! XDP ingress filter: counts IPv4 packets per protocol and drops ICMP with
//! XDP_DROP — in the driver, before an sk_buff exists. Uses raw data/data_end
//! pointers with an explicit bounds check (ptr_at) so the verifier accepts
//! every access.

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
#[map] static PKTS:  HashMap<u32, u64> = HashMap::with_max_entries(16, 0);
#[map] static DROPS: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);

/// Return a pointer to a `T` at `offset` only after proving it lies within
/// the packet window [data, data_end). This is the bounds proof the verifier
/// requires before any dereference.
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
    let new = unsafe { m.get(&key).copied().unwrap_or(0) } + by;
    let _ = m.insert(&key, &new, 0);
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
        bump(&DROPS, proto as u32, 1);
        return Ok(xdp_action::XDP_DROP); // dropped in the driver, before the stack
    }
    Ok(xdp_action::XDP_PASS)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
