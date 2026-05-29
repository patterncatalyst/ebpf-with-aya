//! Types shared between the kernel (`hello-ebpf`) and user space (`hello`).
//!
//! For hello-world we only share a couple of constants: the program counts
//! events into a one-slot per-CPU array. Real programs put `#[repr(C)]`
//! event structs here so a record written by the kernel deserializes
//! correctly in user space.
#![no_std]

/// Index of the single counter slot in the EVENTS per-CPU array.
pub const EVENTS_INDEX: u32 = 0;

/// Number of slots in the EVENTS map.
pub const EVENTS_LEN: u32 = 1;
