/* REFERENCE (canonical C) — emit VARIABLE-LENGTH records through a ring-buffer
 * dynptr: reserve exactly `len` bytes, fill, submit. The aya rendering reserves
 * a fixed max (dynptr reserve is emerging in aya). Needs kernel >= 5.19. */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>

struct hdr { __u32 pid; __u32 len; };

struct {
    __uint(type, BPF_MAP_TYPE_RINGBUF);
    __uint(max_entries, 1 << 16);
} rb SEC(".maps");

SEC("tracepoint/syscalls/sys_enter_getpid")
int emit(void *ctx)
{
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u32 plen = (pid & 1) ? 48 : 8;             /* runtime-varying payload */
    __u32 len = sizeof(struct hdr) + plen;
    struct bpf_dynptr d;

    if (bpf_ringbuf_reserve_dynptr(&rb, len, 0, &d) == 0) {
        struct hdr *h = bpf_dynptr_data(&d, 0, sizeof(*h)); /* checked slice */
        if (h) { h->pid = pid; h->len = len; }
    }
    bpf_ringbuf_submit_dynptr(&d, 0);            /* slice now invalid */
    return 0;
}

char _license[] SEC("license") = "GPL";
