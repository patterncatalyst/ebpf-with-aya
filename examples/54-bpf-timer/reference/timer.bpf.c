/* REFERENCE (canonical C) — in-kernel per-second aggregation via a self-
 * rescheduling bpf_timer. The Aya rendering in timer-ebpf mirrors this but the
 * callback-as-subprogram ergonomics are still rough; this C is authoritative.
 * Needs kernel >= 5.15. */
#include <vmlinux.h>
#include <bpf/bpf_helpers.h>

#define NSEC_PER_SEC 1000000000ULL

struct slot { __u64 count; __u64 rate; struct bpf_timer timer; };

struct {
    __uint(type, BPF_MAP_TYPE_ARRAY);
    __uint(max_entries, 1);
    __type(key, __u32);
    __type(value, struct slot);
} slots SEC(".maps");

/* runs in softirq every second; snapshots the window and re-arms itself */
static int tick(void *map, __u32 *key, struct slot *s)
{
    s->rate = s->count;
    s->count = 0;
    bpf_timer_start(&s->timer, NSEC_PER_SEC, 0); /* periodic */
    return 0;
}

SEC("tracepoint/syscalls/sys_enter_getpid")
int count(void *ctx)
{
    __u32 k = 0;
    struct slot *s = bpf_map_lookup_elem(&slots, &k);
    if (!s) return 0;
    __sync_fetch_and_add(&s->count, 1);
    return 0;
}

/* one-time: initialize, set callback, arm the first fire */
SEC("tracepoint/syscalls/sys_enter_execve")
int arm(void *ctx)
{
    __u32 k = 0;
    struct slot *s = bpf_map_lookup_elem(&slots, &k);
    if (!s) return 0;
    bpf_timer_init(&s->timer, &slots, CLOCK_MONOTONIC);
    bpf_timer_set_callback(&s->timer, tick);
    bpf_timer_start(&s->timer, NSEC_PER_SEC, 0);
    return 0;
}

char _license[] SEC("license") = "GPL";
