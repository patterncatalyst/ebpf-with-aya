/* REFERENCE (canonical C) — a tiny BPF object whose only purpose is to be fed to
 * `bpftool gen skeleton -L`, so we can read the BPF_PROG_TYPE_SYSCALL *loader
 * program* bpftool generates. It needs a BTF-defined map (the `.maps` section)
 * and one program; the light skeleton then embeds a syscall program that creates
 * the map and loads the prog entirely in-kernel.
 *
 * Why not an aya object? aya-ebpf emits legacy `maps`-section map definitions,
 * which libbpf v1.0+ (and thus `bpftool gen skeleton`) refuses to open
 * ("legacy map definitions in 'maps' section are not supported"). Skeletons are
 * a libbpf concept; aya is its own loader, so we compile a libbpf-style C object
 * here to inspect the light skeleton. */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>

struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, __u64);
} exec_count SEC(".maps");

SEC("tracepoint/syscalls/sys_enter_execve")
int count_execve(void *ctx)
{
    __u32 key = 0;
    __u64 *v = bpf_map_lookup_elem(&exec_count, &key);
    if (v)
        __sync_fetch_and_add(v, 1);
    return 0;
}

char _license[] SEC("license") = "GPL";
