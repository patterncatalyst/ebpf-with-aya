#![no_std]
#![no_main]
//! Per-request latency for nginx via two uprobes, keyed by the request object
//! (ngx_http_request_t *r). req_start stamps a start time; req_done computes
//! the elapsed time and buckets it into a log2(us) histogram. We never
//! dereference r — its pointer value is just a key.

use aya_ebpf::{
    helpers::bpf_ktime_get_ns,
    macros::{map, uprobe},
    maps::HashMap,
    programs::ProbeContext,
};

#[map] static STARTS: HashMap<u64, u64> = HashMap::with_max_entries(10240, 0);
#[map] static HIST: HashMap<u32, u64> = HashMap::with_max_entries(64, 0);

#[inline(always)]
fn log2(mut v: u64) -> u32 {
    let mut r = 0u32;
    while v > 1 {
        v >>= 1;
        r += 1;
    }
    r
}

#[inline(always)]
fn bump(m: &HashMap<u32, u64>, k: u32, by: u64) {
    let n = unsafe { m.get(&k).copied().unwrap_or(0) } + by;
    let _ = m.insert(&k, &n, 0);
}

#[uprobe]
pub fn req_start(ctx: ProbeContext) -> u32 {
    let r: u64 = ctx.arg(0).unwrap_or(0); // ngx_http_request_t *r
    if r != 0 {
        let now = unsafe { bpf_ktime_get_ns() };
        let _ = STARTS.insert(&r, &now, 0);
    }
    0
}

#[uprobe]
pub fn req_done(ctx: ProbeContext) -> u32 {
    let r: u64 = ctx.arg(0).unwrap_or(0);
    if r == 0 {
        return 0;
    }
    if let Some(&start) = unsafe { STARTS.get(&r) } {
        let now = unsafe { bpf_ktime_get_ns() };
        let _ = STARTS.remove(&r);
        if now > start {
            let us = (now - start) / 1000;
            bump(&HIST, log2(us), 1);
        }
    }
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
