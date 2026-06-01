#![no_std]
/// Per-command bucket key (the app process: "python" / "java").
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Comm { pub name: [u8; 16] }
#[cfg(feature = "user")]
unsafe impl aya::Pod for Comm {}
