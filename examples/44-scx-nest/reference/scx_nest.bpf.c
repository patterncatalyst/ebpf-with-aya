/* REFERENCE ONLY — a simplified excerpt of the scx_nest scheduler's CPU
 * selection, shown to convey the policy. The real scx_nest (in Fedora's
 * scx-scheds, runnable as the `scx_nest` binary) is considerably larger:
 * it manages the nest cpumasks with timers for demotion, tracks per-core
 * state, and tunes thresholds. You do not build this file here. */
#include <scx/common.bpf.h>

private(NEST) struct bpf_cpumask __kptr *primary_nest; /* warm cores */
private(NEST) struct bpf_cpumask __kptr *reserve;      /* promotable cores */

s32 BPF_STRUCT_OPS(nest_select_cpu, struct task_struct *p, s32 prev_cpu, u64 wake_flags)
{
	s32 cpu;

	/* 1. Prefer an idle core already in the primary (warm) nest. */
	cpu = scx_bpf_pick_idle_cpu((const struct cpumask *)primary_nest, 0);
	if (cpu >= 0) {
		scx_bpf_dispatch(p, SCX_DSQ_LOCAL, SCX_SLICE_DFL, 0);
		return cpu;
	}

	/* 2. Nest saturated — promote one reserve core into the nest. */
	cpu = scx_bpf_pick_idle_cpu((const struct cpumask *)reserve, 0);
	if (cpu >= 0) {
		bpf_cpumask_set_cpu(cpu, primary_nest);
		scx_bpf_dispatch(p, SCX_DSQ_LOCAL, SCX_SLICE_DFL, 0);
		return cpu;
	}

	/* 3. Everything busy — stay put. */
	return prev_cpu;
}

/* A timer / update_idle path (omitted) clears a core's bit from primary_nest
 * after it has been idle past a threshold, shrinking the nest under low load. */
