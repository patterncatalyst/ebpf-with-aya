//! Shared event for execsnoop. argv is captured into fixed slots (no dynamic
//! offset math) to stay verifier-friendly: up to MAX_ARGS args, each up to
//! ARG_LEN bytes, NUL-terminated. `args_count` says how many slots are valid.
#![no_std]

pub const COMM_LEN: usize = 16;
pub const NAME_LEN: usize = 256;
pub const MAX_ARGS: usize = 8;
pub const ARG_LEN: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ExecEvent {
    pub pid: u32,
    pub uid: u32,
    pub args_count: u32,
    pub comm: [u8; COMM_LEN],
    pub filename: [u8; NAME_LEN],
    pub args: [[u8; ARG_LEN]; MAX_ARGS],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ExecEvent {}
