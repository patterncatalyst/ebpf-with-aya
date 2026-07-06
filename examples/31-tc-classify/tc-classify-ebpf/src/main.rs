#![no_std]
#![no_main]
//! tc egress classifier: counts packets/bytes per L4 protocol and drops
//! traffic to BLOCK_PORT with TC_ACT_SHOT. Aggregates in-kernel (no per-packet
//! ring buffer — egress is a hot path); user space reads the totals on a timer.

use aya_ebpf::{
    bindings::{TC_ACT_OK, TC_ACT_SHOT},
    macros::{classifier, map},
    maps::HashMap,
    programs::TcContext,
};
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{Ipv4Hdr, IpProto},
    tcp::TcpHdr,
    udp::UdpHdr,
};
use tc_classify_common::BLOCK_PORT;

#[map] static PKTS:  HashMap<u32, u64> = HashMap::with_max_entries(16, 0);
#[map] static BYTES: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);
#[map] static DROPS: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);

#[inline(always)]
fn bump(m: &HashMap<u32, u64>, key: u32, by: u64) {
    let new = unsafe { m.get(&key).copied().unwrap_or(0) } + by;
    let _ = m.insert(&key, &new, 0);
}

#[classifier]
pub fn tc_classify(ctx: TcContext) -> i32 {
    // On any parse miss, pass — never break legitimate traffic.
    try_classify(&ctx).unwrap_or(TC_ACT_OK)
}

fn try_classify(ctx: &TcContext) -> Result<i32, ()> {
    let eth: EthHdr = ctx.load(0).map_err(|_| ())?;
    let ether_type = eth.ether_type;
    if ether_type != EtherType::Ipv4 {
        return Ok(TC_ACT_OK);
    }
    let ip: Ipv4Hdr = ctx.load(EthHdr::LEN).map_err(|_| ())?;
    let ip_proto = ip.proto;
    let proto = ip_proto as u32;

    bump(&PKTS, proto, 1);
    bump(&BYTES, proto, ctx.len() as u64);

    let l4 = EthHdr::LEN + Ipv4Hdr::LEN;
    let dport = match ip_proto {
        IpProto::Tcp => u16::from_be(ctx.load::<TcpHdr>(l4).map_err(|_| ())?.dest),
        IpProto::Udp => u16::from_be(ctx.load::<UdpHdr>(l4).map_err(|_| ())?.dest),
        _ => 0,
    };
    if dport == BLOCK_PORT {
        bump(&DROPS, proto, 1);
        return Ok(TC_ACT_SHOT); // the verdict: this packet never reaches the wire
    }
    Ok(TC_ACT_OK)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
