//! Shared event: one goroutine state transition (we record the NEW state).
#![no_std]
pub const COMM_LEN: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GoStateEvent {
    pub pid: u32,       // OS thread (M) pid — NOT the goroutine id
    pub newstate: u32,  // _Grunnable=1, _Grunning=2, _Gwaiting=4, ...
    pub comm: [u8; COMM_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for GoStateEvent {}
