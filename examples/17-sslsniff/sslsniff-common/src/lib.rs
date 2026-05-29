//! Shared event for sslsniff: a chunk of plaintext seen crossing SSL_write or
//! SSL_read, before encryption / after decryption.
#![no_std]
pub const COMM_LEN: usize = 16;
pub const DATA_CAP: usize = 256;

pub const DIR_WRITE: u8 = 0;
pub const DIR_READ: u8 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TlsEvent {
    pub pid: u32,
    pub dir: u8,          // DIR_WRITE | DIR_READ
    pub _pad: [u8; 3],
    pub len: u32,         // bytes actually transferred (may exceed captured)
    pub captured: u32,    // bytes copied into `data`
    pub comm: [u8; COMM_LEN],
    pub data: [u8; DATA_CAP],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for TlsEvent {}
