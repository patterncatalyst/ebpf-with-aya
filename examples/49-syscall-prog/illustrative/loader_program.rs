// ILLUSTRATIVE ONLY — the shape of a BPF_PROG_TYPE_SYSCALL "loader program".
// This is NOT built here: syscall-program authoring in aya-ebpf is experimental,
// and in practice loader programs are *generated* (by bpftool's light skeleton),
// not hand-written. Read it to understand what such a program does.
//
// A loader program runs once (via BPF_PROG_RUN), is sleepable, and replays the
// sequence of bpf() commands a user-space loader would have issued — using the
// syscall-context helpers bpf_sys_bpf / bpf_sys_close / bpf_btf_find_by_name_kind.

#![allow(dead_code)]

const BPF_MAP_CREATE: u32 = 0;
const BPF_PROG_LOAD: u32 = 5;

// Stand-ins for the kernel's bpf_attr union views.
#[repr(C)]
struct MapCreateAttr { map_type: u32, key_size: u32, value_size: u32, max_entries: u32 }
#[repr(C)]
struct ProgLoadAttr { prog_type: u32, insn_cnt: u32, insns: u64, /* … fd_array, btf_fd, … */ }

extern "C" {
    // syscall-context helpers (available to BPF_PROG_TYPE_SYSCALL programs)
    fn bpf_sys_bpf(cmd: u32, attr: *mut core::ffi::c_void, attr_size: u32) -> i64;
    fn bpf_sys_close(fd: i32) -> i64;
}

/// The loader body: create a map, then load a program that uses it. The
/// instructions and map values live in data embedded alongside this program.
pub unsafe fn loader() -> i64 {
    // 1. create the map
    let mut mc = MapCreateAttr { map_type: 1 /*HASH*/, key_size: 4, value_size: 8, max_entries: 1 };
    let map_fd = bpf_sys_bpf(
        BPF_MAP_CREATE,
        &mut mc as *mut _ as *mut core::ffi::c_void,
        core::mem::size_of::<MapCreateAttr>() as u32,
    );
    if map_fd < 0 { return map_fd; }

    // 2. load a program that references the map (insns embedded as data)
    let mut pl = ProgLoadAttr { prog_type: 5 /*TRACEPOINT*/, insn_cnt: 0, insns: 0 };
    let prog_fd = bpf_sys_bpf(
        BPF_PROG_LOAD,
        &mut pl as *mut _ as *mut core::ffi::c_void,
        core::mem::size_of::<ProgLoadAttr>() as u32,
    );

    // 3. clean up the intermediate fd
    bpf_sys_close(map_fd as i32);
    prog_fd
}
