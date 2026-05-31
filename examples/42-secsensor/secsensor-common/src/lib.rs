#![no_std]

pub const ET_EXEC: u32 = 1;
pub const ET_PTRACE: u32 = 2;
pub const ET_SETUID: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SecEvent {
    pub etype: u32,
    pub pid: u32,
    pub comm: [u8; 16],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for SecEvent {}
