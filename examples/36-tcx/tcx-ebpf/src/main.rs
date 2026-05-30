#![no_std]
#![no_main]
//! A minimal tc ingress classifier: count packets per L4 protocol, never
//! disturb traffic (TC_ACT_OK). Identical in shape to Chapter 31 — only the
//! user-space attach (tcx, not clsact) differs.

use aya_ebpf::{
    bindings::TC_ACT_OK,
    macros::{classifier, map},
    maps::HashMap,
    programs::TcContext,
};
use network_types::{
    eth::{EthHdr, EtherType},
    ip::Ipv4Hdr,
};

#[map] static PKTS: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);

#[inline(always)]
fn bump(m: &HashMap<u32, u64>, key: u32, by: u64) {
    let n = unsafe { m.get(&key).copied().unwrap_or(0) } + by;
    let _ = m.insert(&key, &n, 0);
}

#[classifier]
pub fn tcx_count(ctx: TcContext) -> i32 {
    let _ = count(&ctx);
    TC_ACT_OK // observe only
}

fn count(ctx: &TcContext) -> Result<(), ()> {
    let eth: EthHdr = ctx.load(0).map_err(|_| ())?;
    if eth.ether_type != EtherType::Ipv4 {
        return Ok(());
    }
    let ip: Ipv4Hdr = ctx.load(EthHdr::LEN).map_err(|_| ())?;
    bump(&PKTS, ip.proto as u32, 1);
    Ok(())
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
