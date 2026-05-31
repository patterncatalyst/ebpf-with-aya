/* REFERENCE (canonical C) — a minimal Reno-style TCP congestion control as a
 * BPF struct_ops. Register with `bpftool struct_ops register`. struct_ops
 * authoring in aya-ebpf is emerging; this C is authoritative. The kernel calls
 * these programs at the tcp_congestion_ops call sites. */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

#define max(a, b) ((a) > (b) ? (a) : (b))

/* tcp_sock embeds sock at offset 0, so this cast is valid */
static __always_inline struct tcp_sock *tcp_sk(const struct sock *sk)
{
    return (struct tcp_sock *)sk;
}

/* building blocks the CC struct_ops interface exposes to BPF (like kfuncs) */
extern void tcp_slow_start(struct tcp_sock *tp, __u32 acked) __ksym;
extern void tcp_cong_avoid_ai(struct tcp_sock *tp, __u32 w, __u32 acked) __ksym;

SEC("struct_ops/cc_ssthresh")
__u32 BPF_PROG(cc_ssthresh, struct sock *sk)
{
    const struct tcp_sock *tp = tcp_sk(sk);
    return max(tp->snd_cwnd >> 1, 2U);            /* multiplicative decrease */
}

SEC("struct_ops/cc_cong_avoid")
void BPF_PROG(cc_cong_avoid, struct sock *sk, __u32 ack, __u32 acked)
{
    struct tcp_sock *tp = tcp_sk(sk);
    if (tp->snd_cwnd < tp->snd_ssthresh)
        tcp_slow_start(tp, acked);                /* exponential below ssthresh */
    else
        tcp_cong_avoid_ai(tp, tp->snd_cwnd, acked); /* additive above */
}

SEC("struct_ops/cc_undo_cwnd")
__u32 BPF_PROG(cc_undo_cwnd, struct sock *sk)
{
    const struct tcp_sock *tp = tcp_sk(sk);
    return max(tp->snd_cwnd, tp->prior_cwnd);     /* restore after spurious loss */
}

SEC(".struct_ops.link")
struct tcp_congestion_ops bpf_reno = {
    .ssthresh   = (void *)cc_ssthresh,
    .cong_avoid = (void *)cc_cong_avoid,
    .undo_cwnd  = (void *)cc_undo_cwnd,
    .name       = "bpf_reno",
};

char _license[] SEC("license") = "GPL";
