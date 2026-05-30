#![no_std]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KillEvent {
    pub pid: u32,
    pub comm: [u8; 16],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for KillEvent {}
