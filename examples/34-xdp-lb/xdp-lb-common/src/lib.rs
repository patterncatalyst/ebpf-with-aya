#![no_std]

/// The virtual port clients send to; the XDP program rewrites the UDP
/// destination port of matching datagrams to one of the backends.
pub const VIP_PORT: u16 = 8080;
