#![no_std]

/// Human-readable name for an IPv4 protocol number, used as a metric label.
pub fn proto_name(p: u32) -> &'static str {
    match p {
        1 => "icmp",
        6 => "tcp",
        17 => "udp",
        _ => "other",
    }
}
