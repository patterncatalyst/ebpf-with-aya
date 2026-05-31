#![no_std]

/// A variable-length record: `len` says how many bytes are meaningful. The
/// canonical dynptr version (reference/dynptr_ringbuf.bpf.c) reserves exactly
/// `len` bytes; this fixed layout is the closest aya supports today.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Record {
    pub pid: u32,
    pub len: u32,
    pub data: [u8; 64],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for Record {}
