#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ReadCtx {
    pub buf: u64,
    pub count: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Payload {
    pub line: [u8; 64],
    pub len: u32,
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ReadCtx {}
#[cfg(feature = "user")]
unsafe impl aya::Pod for Payload {}
