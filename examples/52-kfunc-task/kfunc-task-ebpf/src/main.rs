#![no_std]
#![no_main]
//! Look up a task by pid from inside a BPF program using the canonical
//! acquire/release kfunc pair, and let the verifier enforce the release.
//!
//! UNVERIFIED: confirm that bpf_task_from_pid / bpf_task_release exist in this
//! kernel's BTF, and the Aya mechanics for declaring/calling kfuncs (extern
//! prototypes resolved via BTF) + the aya-tool-generated task_struct.

mod vmlinux;
use vmlinux::task_struct;

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::{Array, HashMap},
    programs::TracePointContext,
};

// kfuncs are declared as extern fns; the linker/loader resolve them via BTF.
extern "C" {
    fn bpf_task_from_pid(pid: i32) -> *mut task_struct; // KF_ACQUIRE | KF_RET_NULL
    fn bpf_task_release(task: *mut task_struct); //         KF_RELEASE
}

#[map] static CONFIG: Array<u32> = Array::with_max_entries(1, 0); // target pid (set by user space)
#[map] static RESULT: HashMap<u32, u64> = HashMap::with_max_entries(2, 0); // 0=found, 1=missing

#[inline(always)]
fn bump(key: u32) {
    let n = unsafe { RESULT.get(&key).copied().unwrap_or(0) } + 1;
    let _ = RESULT.insert(&key, &n, 0);
}

#[tracepoint]
pub fn lookup(_ctx: TracePointContext) -> u32 {
    let pid = match CONFIG.get(0) {
        Some(&p) => p,
        None => return 0,
    };

    // KF_ACQUIRE: from here the verifier tracks an unreleased task reference
    let task = unsafe { bpf_task_from_pid(pid as i32) };

    // KF_RET_NULL: the verifier forces this null-check before any use
    if task.is_null() {
        bump(1); // missing — nothing was acquired, so returning is fine
        return 0;
    }

    bump(0); // found — we hold a live, trusted task_struct *
    // (here you could read task fields — a trusted-pointer read; see Part 9)

    // KF_RELEASE: mandatory on this path. Delete this line and the verifier
    // rejects the load because the non-null path would leak the reference.
    unsafe { bpf_task_release(task) };
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
