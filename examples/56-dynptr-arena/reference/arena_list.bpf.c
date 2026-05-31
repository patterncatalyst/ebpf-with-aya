/* REFERENCE (canonical C) — build a linked list in a BPF arena with REAL
 * pointers; user space mmaps the same region to read it zero-copy. Compile with
 * -D__BPF_FEATURE_ADDR_SPACE_CAST. Needs kernel >= 6.9. arena support in aya is
 * nascent, so this is the canonical form. */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>

#define NUMA_NO_NODE (-1)

struct {
    __uint(type, BPF_MAP_TYPE_ARENA);
    __uint(map_flags, BPF_F_MMAPABLE);
    __uint(max_entries, 100);                    /* pages */
} arena SEC(".maps");

struct node { struct node __arena *next; __u64 value; };
struct node __arena *head;                       /* arena pointer */

SEC("tracepoint/syscalls/sys_enter_getpid")
int push(void *ctx)
{
    struct node __arena *n =
        bpf_arena_alloc_pages(&arena, NULL, 1, NUMA_NO_NODE, 0);
    if (!n) return 0;
    n->value = bpf_ktime_get_ns();               /* ordinary pointer writes */
    n->next = head;
    head = n;                                    /* push onto the list */
    return 0;
}

char _license[] SEC("license") = "GPL";
