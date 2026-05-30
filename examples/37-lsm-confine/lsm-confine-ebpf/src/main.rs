#![no_std]
#![no_main]
//! BPF LSM: deny socket connect() for processes whose cgroup is in CONFINED.
//! Return 0 to allow, -EPERM (-1) to deny. The cgroup id comes from a plain
//! helper, so there is no kernel-struct walking here.

use aya_ebpf::{
    helpers::bpf_get_current_cgroup_id,
    macros::{lsm, map},
    maps::HashMap,
    programs::LsmContext,
};

#[map] static CONFINED: HashMap<u64, u8> = HashMap::with_max_entries(64, 0);
#[map] static DENIED: HashMap<u32, u64> = HashMap::with_max_entries(1, 0);

#[lsm(hook = "socket_connect")]
pub fn confine_connect(ctx: LsmContext) -> i32 {
    // socket_connect args: (sock, address, addrlen, ret). The trailing `ret`
    // is the running verdict — respect a prior LSM's denial.
    let prior: i32 = unsafe { ctx.arg(3) };
    if prior != 0 {
        return prior;
    }
    let cgid = unsafe { bpf_get_current_cgroup_id() };
    if unsafe { CONFINED.get(&cgid) }.is_some() {
        let k = 0u32;
        let n = unsafe { DENIED.get(&k).copied().unwrap_or(0) } + 1;
        let _ = DENIED.insert(&k, &n, 0);
        return -1; // -EPERM
    }
    0 // allow
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
