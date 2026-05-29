//! Shared constants for runqlat. No per-event struct: the kernel aggregates a
//! log2-microsecond histogram in an Array map; user space reads the buckets.
#![no_std]

/// Number of log2(microseconds) buckets. Bucket i counts delays in
/// [2^i, 2^(i+1)) microseconds. 27 buckets -> up to ~134 s.
pub const NBUCKETS: u32 = 27;
