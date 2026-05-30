//! Shared event for sockops: a TCP connection that just became established,
//! with its direction and 4-tuple (provided directly by the sock_ops context).
#![no_std]

pub const DIR_ACTIVE: u8 = 0;  // we initiated (connect)
pub const DIR_PASSIVE: u8 = 1; // we accepted

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SockEvent {
    pub local_ip4: u32,  // network byte order
    pub remote_ip4: u32, // network byte order
    pub local_port: u16, // host byte order (sock_ops convention)
    pub remote_port: u16,// network byte order (sock_ops convention)
    pub dir: u8,
    pub _pad: [u8; 3],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for SockEvent {}
