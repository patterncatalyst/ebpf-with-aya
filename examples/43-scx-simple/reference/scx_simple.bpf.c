/* REFERENCE ONLY — this is the shape of the upstream scx_simple scheduler's
 * BPF callbacks (C), shown so you can read the policy. On Fedora it ships
 * prebuilt in the `scx-scheds` package as the `scx_simple` binary; you do not
 * build this file here. See the chapter's note on why sched_ext schedulers are
 * written in C today while user space is Rust. */
#include <scx/common.bpf.h>

char _license[] SEC("license") = "GPL";
UEI_DEFINE(uei);

s32 BPF_STRUCT_OPS(simple_select_cpu, struct task_struct *p, s32 prev_cpu, u64 wake_flags)
{
	bool is_idle = false;
	s32 cpu = scx_bpf_select_cpu_dfl(p, prev_cpu, wake_flags, &is_idle);
	if (is_idle)
		scx_bpf_dispatch(p, SCX_DSQ_LOCAL, SCX_SLICE_DFL, 0); /* run here now */
	return cpu;
}

void BPF_STRUCT_OPS(simple_enqueue, struct task_struct *p, u64 enq_flags)
{
	/* park on the global FIFO; the kernel drains it onto idle CPUs */
	scx_bpf_dispatch(p, SCX_DSQ_GLOBAL, SCX_SLICE_DFL, enq_flags);
}

s32 BPF_STRUCT_OPS_SLEEPABLE(simple_init) { return 0; }
void BPF_STRUCT_OPS(simple_exit, struct scx_exit_info *ei) { UEI_RECORD(uei, ei); }

SCX_OPS_DEFINE(simple_ops,
	       .select_cpu = (void *)simple_select_cpu,
	       .enqueue    = (void *)simple_enqueue,
	       .init       = (void *)simple_init,
	       .exit       = (void *)simple_exit,
	       .name       = "simple");
