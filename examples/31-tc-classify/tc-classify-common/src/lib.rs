#![no_std]

/// Egress traffic to this L4 destination port is dropped (TC_ACT_SHOT) to
/// demonstrate that the verdict is real. Change and rebuild to block another.
pub const BLOCK_PORT: u16 = 9999;

/// Human-readable name for an IPv4 protocol number, used as a metric label.
pub fn proto_name(p: u32) -> &'static str {
    match p {
        1 => "icmp",
        6 => "tcp",
        17 => "udp",
        _ => "other",
    }
}
