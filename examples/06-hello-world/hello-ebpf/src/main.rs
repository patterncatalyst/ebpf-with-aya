//! hello-ebpf — the in-kernel half of hello-world.
//!
//! Attaches (from user space) to the `sys_enter_execve` tracepoint, so it
//! fires every time any process on the target calls execve(). On each hit it
//! bumps a per-CPU counter and emits one aya-log line.
//!
//! Built for the BPF target; no_std, no_main.
#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::PerCpuArray,
    programs::TracePointContext,
};
use aya_log_ebpf::info;
use hello_common::{EVENTS_INDEX, EVENTS_LEN};

/// One-slot per-CPU counter. User space sums across CPUs.
#[map]
static EVENTS: PerCpuArray<u64> = PerCpuArray::with_max_entries(EVENTS_LEN, 0);

#[tracepoint]
pub fn hello(ctx: TracePointContext) -> u32 {
    match try_hello(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_hello(ctx: TracePointContext) -> Result<u32, u32> {
    // Increment the per-CPU counter at index 0.
    if let Some(counter) = EVENTS.get_ptr_mut(EVENTS_INDEX) {
        unsafe {
            *counter += 1;
        }
    }
    info!(&ctx, "hello: execve observed");
    Ok(0)
}

// Required license declaration so the program may call GPL-only helpers
// (aya-log uses bpf_trace_printk-style helpers under the hood).
#[link_section = "license"]
#[no_mangle]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

// Newer aya-ebpf provides a panic handler; if your toolchain complains about a
// duplicate, delete this block. Kept explicit for clarity.
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
