//! Shared event for the L7 socket filter: a captured HTTP request/response line
//! plus the IPv4 4-tuple it came from.
#![no_std]
pub const LINE_CAP: usize = 80;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct HttpEvent {
    pub saddr: u32, // network byte order
    pub daddr: u32,
    pub sport: u16, // network byte order
    pub dport: u16,
    pub len: u32,
    pub line: [u8; LINE_CAP],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for HttpEvent {}
