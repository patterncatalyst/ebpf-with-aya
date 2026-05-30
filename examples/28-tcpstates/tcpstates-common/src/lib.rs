//! Shared event for tcpstates: one TCP state transition, with the endpoints —
//! all delivered by the sock:inet_sock_set_state tracepoint (no struct reads).
#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TcpStateEvent {
    pub saddr: [u8; 4],
    pub daddr: [u8; 4],
    pub sport: u16,
    pub dport: u16,
    pub oldstate: u32,
    pub newstate: u32,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for TcpStateEvent {}
