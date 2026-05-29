//! A tiny program to be probed by the uprobe in this chapter.
//!
//! `compute` is exported with #[no_mangle] + extern "C" so the symbol is
//! literally "compute" (not a mangled Rust symbol) and uses the C calling
//! convention (first arg in rdi on x86_64), which makes ctx.arg(0) in the
//! uprobe read it cleanly. #[inline(never)] guarantees a real call site to
//! attach to.
use std::{thread, time::Duration};

#[no_mangle]
#[inline(never)]
pub extern "C" fn compute(x: u64) -> u64 {
    // arbitrary work so the call isn't optimized away
    x.wrapping_mul(2654435761) ^ (x >> 13)
}

fn main() {
    println!("target-app pid {} — calling compute() every 500ms", std::process::id());
    let mut i: u64 = 0;
    loop {
        let r = compute(i);
        println!("compute({i}) = {r}");
        i += 1;
        thread::sleep(Duration::from_millis(500));
    }
}
