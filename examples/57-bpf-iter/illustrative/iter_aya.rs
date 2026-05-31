// ILLUSTRATIVE — the shape a BPF iterator takes in aya-ebpf. Iterator program
// support in aya is EMERGING; this does not build today. The canonical form is
// reference/task_iter.bpf.c, pinned with `bpftool iter pin`. Read for direction.
//
// The model is identical to the C: a program marked for an iterator target,
// called once per element, emitting through the seq_file in ctx.meta.

// #[iter("task")]                                  // (emerging attribute, shape only)
// pub fn dump_task(ctx: IterContext<TaskIter>) -> i32 {
//     let seq = ctx.meta().seq();
//     match ctx.element() {
//         None => { seq.printf!("total: {} tasks\n", count); }     // end
//         Some(task) => {
//             if ctx.meta().seq_num() == 0 { seq.printf!("TGID PID COMM\n"); } // header
//             seq.printf!("{} {} {}\n", task.tgid, task.pid, task.comm());
//         }
//     }
//     0
// }
//
// Until this lands, pin the C object with bpftool (see demo.sh): the kernel
// drives the loop, your program is the per-element body, output is a seq_file.
