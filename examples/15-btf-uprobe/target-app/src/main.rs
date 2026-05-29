//! Target program: calls process_order() with a pointer to an Order struct.
//! Built with debug info so it carries type information (DWARF) that
//! `pahole -J` can turn into BTF.
use btf_uprobe_common::Order;
use std::{thread, time::Duration};

#[no_mangle]
#[inline(never)]
pub extern "C" fn process_order(order: *const Order) -> u64 {
    // SAFETY: caller passes a valid pointer.
    let o = unsafe { &*order };
    o.amount_cents // trivial work so it isn't optimized away
}

fn main() {
    println!("target-app pid {} — submitting orders every 500ms", std::process::id());
    let mut id: u64 = 1000;
    loop {
        let order = Order { id, amount_cents: (id % 7) * 999, status: (id % 3) as u32 };
        let _ = process_order(&order);
        id += 1;
        thread::sleep(Duration::from_millis(500));
    }
}
