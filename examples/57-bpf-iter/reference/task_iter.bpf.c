/* REFERENCE (canonical C) — a BPF iterator over every task: the kernel calls
 * dump_task once per task_struct, we read fields directly and emit a process
 * table through the seq_file. Pin with `bpftool iter pin`, read with `cat`.
 * iterator program support in aya is emerging; this C is authoritative. */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>

char _license[] SEC("license") = "GPL";

unsigned long count = 0;

SEC("iter/task")
int dump_task(struct bpf_iter__task *ctx)
{
    struct seq_file *seq = ctx->meta->seq;
    struct task_struct *task = ctx->task;

    if (task == NULL) {                          /* end of iteration: summary */
        BPF_SEQ_PRINTF(seq, "total: %lu tasks\n", count);
        return 0;
    }
    if (ctx->meta->seq_num == 0)                 /* first call: header */
        BPF_SEQ_PRINTF(seq, "%-8s %-8s %s\n", "TGID", "PID", "COMM");

    /* read kernel fields directly; filter here if you like, e.g.
       if (task->tgid != task->pid) return 0;  // main threads only */
    BPF_SEQ_PRINTF(seq, "%-8d %-8d %s\n", task->tgid, task->pid, task->comm);
    count++;
    return 0;
}
