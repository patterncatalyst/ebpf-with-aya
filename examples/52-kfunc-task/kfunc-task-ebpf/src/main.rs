#![no_std]
#![no_main]
//! "Is the target pid the running task?" — the aya-friendly rendering.
//!
//! The canonical form (reference/task.bpf.c) looks up an ARBITRARY pid's
//! task_struct with the acquire/release kfunc pair bpf_task_from_pid /
//! bpf_task_release, and lets the verifier enforce KF_RELEASE. **aya-ebpf can't
//! do that**: kfuncs are resolved by a BTF-based *call relocation* that
//! aya-ebpf / bpf-linker don't emit for plain `extern "C"` prototypes — the
//! program fails to load with `error relocating 'lookup': function not found` —
//! and aya-ebpf has no `#[kfunc]` mechanism to declare one. So we cannot look up
//! an arbitrary pid's task from the kernel side at all.
//!
//! Rust-based fill-in: check whether the CURRENT task (the process that fired
//! the traced syscall) is the target pid, via the stable `bpf_get_current_pid_tgid`
//! helper — no kfunc, no task_struct field offsets. The loader drives the traced
//! syscall from the target process, so this still tallies found (target seen
//! running) vs missing (a pid that never runs it). The trusted-task-pointer
//! field reads the kfunc chapter wanted are shown with CO-RE in Part 9; the C
//! reference keeps the true kfunc form.

use aya_ebpf::{
    helpers::bpf_get_current_pid_tgid,
    macros::{map, tracepoint},
    maps::{Array, HashMap},
    programs::TracePointContext,
};

#[map] static CONFIG: Array<u32> = Array::with_max_entries(1, 0); // target pid (set by user space)
#[map] static RESULT: HashMap<u32, u64> = HashMap::with_max_entries(2, 0); // 0=found, 1=missing

#[inline(always)]
fn bump(key: u32) {
    let n = unsafe { RESULT.get(&key).copied().unwrap_or(0) } + 1;
    let _ = RESULT.insert(&key, &n, 0);
}

#[tracepoint]
pub fn lookup(_ctx: TracePointContext) -> u32 {
    let target = match CONFIG.get(0) {
        Some(&p) => p,
        None => return 0,
    };
    // The pid (tgid) of the task currently running this syscall — the piece we
    // *can* get without a kfunc. Arbitrary-pid lookup would need bpf_task_from_pid.
    let tgid = (bpf_get_current_pid_tgid() >> 32) as u32;
    if tgid == target {
        bump(0); // found — the target pid is the running task
    } else {
        bump(1); // missing — a different task is running (or the target never runs)
    }
    0
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
