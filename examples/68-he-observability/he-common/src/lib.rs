//! Shared record: one completed homomorphic operation, timed.
//! Note what is NOT here — no operand, no ciphertext, no value. Only which
//! operation ran (`op`) and how long it took (`dur_ns`). The observer is
//! data-blind by construction.
#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sample {
    pub op: u32,      // 0 keygen, 1 encrypt, 2 compute, 3 decrypt
    pub _pad: u32,
    pub dur_ns: u64,  // entry -> return, nanoseconds
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for Sample {}
