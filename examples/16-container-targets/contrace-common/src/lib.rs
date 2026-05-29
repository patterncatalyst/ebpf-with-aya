//! Shared event for contrace: a file-open observed inside a specific container,
//! tagged with the cgroup id it came from.
#![no_std]
pub const COMM_LEN: usize = 16;
pub const NAME_LEN: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ContainerEvent {
    pub pid: u32,        // host/VM PID (see chapter: not the in-container PID)
    pub cgroup: u64,     // bpf_get_current_cgroup_id()
    pub comm: [u8; COMM_LEN],
    pub filename: [u8; NAME_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ContainerEvent {}
