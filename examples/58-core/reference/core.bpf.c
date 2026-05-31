/* REFERENCE (canonical C) — portable field reads with CO-RE. Compiled against a
 * generated vmlinux.h, this object carries BTF *relocations* (descriptions of
 * the fields, by name), which the loader patches against the target kernel's
 * BTF at load. Needs kernel >= 5.8 with CONFIG_DEBUG_INFO_BTF. */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_core_read.h>

struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, __u64);
} reads SEC(".maps");

SEC("kprobe/__x64_sys_getpid")
int BPF_KPROBE(count_reads)
{
    struct task_struct *task = (struct task_struct *)bpf_get_current_task();

    /* FIELD_OFFSET relocation: "field pid in task_struct" — patched at load */
    __u32 pid = BPF_CORE_READ(task, pid);

    /* nested relocation chain, each hop resolved from BTF */
    __u32 inum = BPF_CORE_READ(task, nsproxy, pid_ns_for_children, ns.inum);

    /* FIELD_EXISTS relocation: only read loginuid where the kernel has it */
    if (bpf_core_field_exists(task->loginuid)) {
        __u32 lu = BPF_CORE_READ(task, loginuid.val);
        (void)lu;
    }
    (void)pid; (void)inum;

    __u32 k = 0;
    __u64 *c = bpf_map_lookup_elem(&reads, &k);
    if (c) __sync_fetch_and_add(c, 1);
    return 0;
}

char _license[] SEC("license") = "GPL";
