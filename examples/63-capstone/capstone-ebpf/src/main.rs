#![no_std]
#![no_main]
//! The fourth view: count socket read syscalls per command, so the kernel-side
//! truth of a request (how many reads on the python/java processes) sits beside
//! the app spans. The runnable observer correlates by command/time; the
//! canonical L7 trace_id extraction is in reference/l7_traceparent.bpf.c.

use aya_ebpf::{macros::{map, tracepoint}, maps::HashMap, programs::TracePointContext};
use capstone_common::Comm;

#[map] static SYSCALLS: HashMap<Comm, u64> = HashMap::with_max_entries(1024, 0);

#[tracepoint]
pub fn on_read(ctx: TracePointContext) -> u32 {
    // sys_enter_read: bump the calling command's counter. We read the current
    // comm via the helper rather than the tracepoint args.
    let mut name = [0u8; 16];
    if aya_ebpf::helpers::bpf_get_current_comm().map(|c| name = c).is_ok() {
        let key = Comm { name };
        let cur = unsafe { SYSCALLS.get(&key) }.copied().unwrap_or(0);
        let _ = SYSCALLS.insert(&key, &(cur + 1), 0);
    }
    let _ = &ctx;
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
