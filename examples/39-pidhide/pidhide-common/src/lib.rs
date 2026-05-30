#![no_std]

/// The PID-to-hide, as a null-padded ASCII string (e.g. b"1234\0...").
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PidName(pub [u8; 16]);

#[cfg(feature = "user")]
unsafe impl aya::Pod for PidName {}
