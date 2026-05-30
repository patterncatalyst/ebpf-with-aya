#![no_std]

/// A captured TCP control packet. Addresses are kept in network byte order;
/// ports and length are stored host-order by the kernel side.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FlowRecord {
    pub saddr: u32,
    pub daddr: u32,
    pub sport: u16,
    pub dport: u16,
    pub flags: u8,
    pub len: u16,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for FlowRecord {}

pub const TCP_FIN: u8 = 0x01;
pub const TCP_SYN: u8 = 0x02;
pub const TCP_RST: u8 = 0x04;
pub const TCP_ACK: u8 = 0x10;

pub fn proto_name(p: u32) -> &'static str {
    match p {
        1 => "icmp",
        6 => "tcp",
        17 => "udp",
        _ => "other",
    }
}
