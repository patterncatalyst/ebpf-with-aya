//! Shared map-value: per-task on-CPU time. Energy is attributed in user space
//! by each task's share of total CPU time (Kepler's utilization model), times
//! the system power read from RAPL when available.
#![no_std]
pub const COMM_LEN: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskStat {
    pub cpu_ns: u64,
    pub comm: [u8; COMM_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for TaskStat {}
