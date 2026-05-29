//! biopattern-ebpf — classify block I/O as sequential vs random, per device.
//!
//! On block:block_rq_issue we read (dev, sector, nr_sector). If this request's
//! starting sector equals where the previous request on the same device ended,
//! it's SEQUENTIAL; otherwise RANDOM. We track the last end-sector per device
//! and accumulate per-device counters in kernel.
#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::HashMap,
    programs::TracePointContext,
};
use biopattern_common::BioStat;

// block_rq_issue format (verify via the format file):
//   unsigned int dev   @ 8
//   unsigned long long sector @ 16
//   unsigned int nr_sector @ 24
const OFF_DEV: usize = 8;
const OFF_SECTOR: usize = 16;
const OFF_NR_SECTOR: usize = 24;
const SECTOR_BYTES: u64 = 512;

#[map] static STATS: HashMap<u32, BioStat> = HashMap::with_max_entries(256, 0);
#[map] static LAST_END: HashMap<u32, u64> = HashMap::with_max_entries(256, 0);

#[tracepoint]
pub fn block_rq_issue(ctx: TracePointContext) -> u32 {
    let _ = try_bio(&ctx);
    0
}

fn try_bio(ctx: &TracePointContext) -> Result<(), i64> {
    let dev = unsafe { ctx.read_at::<u32>(OFF_DEV) }.map_err(|_| 0i64)?;
    let sector = unsafe { ctx.read_at::<u64>(OFF_SECTOR) }.map_err(|_| 0i64)?;
    let nr_sector = unsafe { ctx.read_at::<u32>(OFF_NR_SECTOR) }.map_err(|_| 0i64)? as u64;

    let is_seq = match unsafe { LAST_END.get(&dev) } {
        Some(&end) => sector == end,
        None => false,
    };
    let _ = LAST_END.insert(&dev, &(sector + nr_sector), 0);

    let cur = unsafe { STATS.get(&dev) }.copied().unwrap_or(BioStat { sequential: 0, random: 0, bytes: 0 });
    let updated = BioStat {
        sequential: cur.sequential + if is_seq { 1 } else { 0 },
        random: cur.random + if is_seq { 0 } else { 1 },
        bytes: cur.bytes + nr_sector * SECTOR_BYTES,
    };
    let _ = STATS.insert(&dev, &updated, 0);
    Ok(())
}

#[link_section = "license"]
#[no_mangle]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
