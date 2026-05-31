#![no_std]
#![no_main]
//! Count execve() calls into a map declared *pinned by name*, so the map (and,
//! with a pinned link, the attachment) survive the loader exiting.

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::HashMap,
    programs::TracePointContext,
};

// `pinned` sets LIBBPF_PIN_BY_NAME: combined with the loader's map_pin_path,
// the map is pinned at /sys/fs/bpf/<dir>/EXECS and reused if already present.
#[map] static EXECS: HashMap<u32, u64> = HashMap::pinned(1, 0);

#[tracepoint]
pub fn count_exec(_ctx: TracePointContext) -> u32 {
    let key = 0u32;
    let n = unsafe { EXECS.get(&key).copied().unwrap_or(0) } + 1;
    let _ = EXECS.insert(&key, &n, 0);
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
