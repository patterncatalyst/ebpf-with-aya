//! memleak-ebpf — pair malloc/calloc with free to find outstanding allocations.
//!
//!   malloc(size)  : entry stash size; return record ALLOCS[ptr] = {size, stack}
//!   calloc(n,sz)  : entry stash n*sz;  return same
//!   free(ptr)     : entry delete ALLOCS[ptr]
//!
//! The allocation's user stack is captured with bpf_get_stackid at return, so
//! every outstanding entry remembers WHERE it was allocated. Filtered to one
//! pid (libc malloc fires constantly) via TARGET_PID.
#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::BPF_F_USER_STACK,
    helpers::bpf_get_current_pid_tgid,
    macros::{map, uprobe, uretprobe},
    maps::{Array, HashMap, StackTrace},
    programs::{ProbeContext, RetProbeContext},
};
use memleak_common::AllocInfo;

#[map] static SIZES: HashMap<u64, u64> = HashMap::with_max_entries(10240, 0);
#[map] static ALLOCS: HashMap<u64, AllocInfo> = HashMap::with_max_entries(1_000_000, 0);
#[map] static STACKS: StackTrace = StackTrace::with_max_entries(16384, 0);
#[map] static TARGET_PID: Array<u32> = Array::with_max_entries(1, 0);

fn skip(pid: u32) -> bool {
    let target = TARGET_PID.get(0).copied().unwrap_or(0);
    target != 0 && pid != target
}

fn on_alloc_enter(size: u64) {
    let id = bpf_get_current_pid_tgid();
    if skip((id >> 32) as u32) { return; }
    let _ = SIZES.insert(&id, &size, 0);
}

fn on_alloc_return(ctx: &RetProbeContext) {
    let id = bpf_get_current_pid_tgid();
    let pid = (id >> 32) as u32;
    if skip(pid) { return; }
    let size = match unsafe { SIZES.get(&id) } { Some(s) => *s, None => return };
    let _ = SIZES.remove(&id);
    let ptr: u64 = ctx.ret().unwrap_or(0);
    if ptr == 0 { return; }
    let stackid = unsafe { STACKS.get_stackid(ctx, BPF_F_USER_STACK as u64) }.unwrap_or(-1) as i32;
    let _ = ALLOCS.insert(&ptr, &AllocInfo { size, stackid, pid }, 0);
}

#[uprobe]
pub fn malloc_enter(ctx: ProbeContext) -> u32 {
    on_alloc_enter(ctx.arg(0).unwrap_or(0));
    0
}

#[uretprobe]
pub fn malloc_exit(ctx: RetProbeContext) -> u32 {
    on_alloc_return(&ctx);
    0
}

#[uprobe]
pub fn calloc_enter(ctx: ProbeContext) -> u32 {
    let nmemb: u64 = ctx.arg(0).unwrap_or(0);
    let size: u64 = ctx.arg(1).unwrap_or(0);
    // Saturating n*sz without a 128-bit widening multiply (unsupported on BPF).
    let limit = core::hint::black_box(if size != 0 { u64::MAX / size } else { u64::MAX });
    let total = if nmemb > limit { u64::MAX } else { nmemb.wrapping_mul(size) };
    on_alloc_enter(total);
    0
}

#[uretprobe]
pub fn calloc_exit(ctx: RetProbeContext) -> u32 {
    on_alloc_return(&ctx);
    0
}

#[uprobe]
pub fn free_enter(ctx: ProbeContext) -> u32 {
    let id = bpf_get_current_pid_tgid();
    if skip((id >> 32) as u32) { return 0; }
    let ptr: u64 = ctx.arg(0).unwrap_or(0);
    if ptr != 0 { let _ = ALLOCS.remove(&ptr); }
    0
}

#[link_section = "license"]
#[no_mangle]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
