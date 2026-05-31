#![no_std]

/// One completed request as seen at the socket layer: how long the service
/// took (recv -> send) and which process handled it (java, python3.14, ...).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Req {
    pub dur_ns: u64,
    pub comm: [u8; 16],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for Req {}
