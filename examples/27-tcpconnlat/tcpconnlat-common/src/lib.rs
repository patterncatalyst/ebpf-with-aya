//! Shared types for tcpconnlat.
#![no_std]
pub const COMM_LEN: usize = 16;

/// Stashed at connect() entry, keyed by the struct sock* pointer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ConnStart {
    pub ts: u64,
    pub pid: u32,
    pub daddr: u32, // IPv4 dest, network byte order
    pub dport: u16, // network byte order
    pub _pad: u16,
    pub comm: [u8; COMM_LEN],
}

/// Emitted when the connection is established.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ConnEvent {
    pub pid: u32,
    pub daddr: u32,
    pub dport: u16,
    pub _pad: u16,
    pub lat_ns: u64,
    pub comm: [u8; COMM_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ConnEvent {}
