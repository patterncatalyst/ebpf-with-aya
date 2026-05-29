//! Target with a function of variable latency, so funclatency produces an
//! interesting distribution.
use std::{thread, time::Duration};

#[no_mangle]
#[inline(never)]
pub extern "C" fn slow_op(n: u64) -> u64 {
    // latency varies with n's low bits: a spread for the histogram
    let micros = 200 + (n % 7) * 400;
    thread::sleep(Duration::from_micros(micros));
    n.wrapping_mul(2654435761)
}

fn main() {
    println!("target-app pid {} — calling slow_op() in a loop", std::process::id());
    let mut n = 0u64;
    loop {
        let _ = slow_op(n);
        n += 1;
        thread::sleep(Duration::from_millis(50));
    }
}
