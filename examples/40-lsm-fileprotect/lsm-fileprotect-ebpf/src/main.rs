#![no_std]
#![no_main]
//! BPF LSM tamper protection: deny MAY_WRITE access to one protected inode,
//! even for root. Reads inode->i_ino with bpf_probe_read_kernel at a
//! version-specific offset (CO-RE, Part 9, makes this portable).

use aya_ebpf::{
    helpers::bpf_probe_read_kernel,
    macros::{lsm, map},
    maps::{Array, HashMap},
    programs::LsmContext,
};

#[map] static PROTECTED: Array<u64> = Array::with_max_entries(1, 0);
#[map] static DENIED: HashMap<u32, u64> = HashMap::with_max_entries(1, 0);

const MAY_WRITE: i32 = 0x02;
// Offset of i_ino within struct inode. VERSION-SPECIFIC — verify with
// `pahole struct inode` / BTF; CO-RE (Part 9) computes it at load time.
const I_INO_OFFSET: usize = 40;

#[inline(always)]
fn bump(m: &HashMap<u32, u64>, k: u32, by: u64) {
    let n = unsafe { m.get(&k).copied().unwrap_or(0) } + by;
    let _ = m.insert(&k, &n, 0);
}

#[lsm(hook = "inode_permission")]
pub fn protect_file(ctx: LsmContext) -> i32 {
    // inode_permission args: (inode, mask, ret)
    let prior: i32 = unsafe { ctx.arg(2) };
    if prior != 0 {
        return prior;
    }
    let mask: i32 = unsafe { ctx.arg(1) };
    if mask & MAY_WRITE == 0 {
        return 0; // only police writes
    }
    let inode: *const u8 = unsafe { ctx.arg(0) };
    let i_ino: u64 = match unsafe { bpf_probe_read_kernel(inode.add(I_INO_OFFSET) as *const u64) } {
        Ok(v) => v,
        Err(_) => return 0, // fail open — never wedge the system on a read error
    };
    let protected = unsafe { PROTECTED.get(0).copied() }.unwrap_or(0);
    if protected != 0 && i_ino == protected {
        bump(&DENIED, 0, 1);
        return -1; // -EPERM
    }
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
