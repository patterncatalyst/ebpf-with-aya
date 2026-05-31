/* REFERENCE (canonical C) — the user ring buffer consumer side. Read this as
 * the authoritative shape; the Aya rendering in user-rb-ebpf is a sketch while
 * Aya's user-ringbuf + dynptr wrappers settle. Needs kernel >= 6.1. */
#include <vmlinux.h>
#include <bpf/bpf_helpers.h>

struct sample { __u64 value; };

struct {
    __uint(type, BPF_MAP_TYPE_USER_RINGBUF);
    __uint(max_entries, 256 * 1024);
} user_rb SEC(".maps");

__u64 total_count = 0;
__u64 total_sum = 0;

/* invoked once per submitted sample; the sample arrives as a dynptr */
static long on_sample(struct bpf_dynptr *dynptr, void *ctx)
{
    struct sample *s = bpf_dynptr_data(dynptr, 0, sizeof(*s));
    if (!s)
        return 0;
    __sync_fetch_and_add(&total_count, 1);
    __sync_fetch_and_add(&total_sum, s->value);
    return 0; /* return 1 to stop draining early (backpressure) */
}

/* drain whenever the program runs; here triggered by getpid() from the loader */
SEC("tracepoint/syscalls/sys_enter_getpid")
int drain_it(void *ctx)
{
    bpf_user_ringbuf_drain(&user_rb, on_sample, NULL, 0);
    return 0;
}

char _license[] SEC("license") = "GPL";
