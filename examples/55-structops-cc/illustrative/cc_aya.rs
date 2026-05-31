// ILLUSTRATIVE — the shape a struct_ops congestion-control algorithm would take
// in aya-ebpf. struct_ops authoring in Aya is EMERGING; this does not build
// today. The canonical implementation is reference/cc.bpf.c, registered with
// `bpftool struct_ops register`. Read this for where Aya is heading.
//
// The idea maps 1:1 to the C: one program per ops slot, gathered into a
// struct_ops map matched to the kernel's `tcp_congestion_ops` by BTF.

// #[struct_ops("tcp_congestion_ops")]            // (emerging attribute, shape only)
// pub struct BpfReno;
//
// impl BpfReno {
//     #[struct_ops_method] fn ssthresh(sk: *mut sock) -> u32 { /* halve cwnd */ 0 }
//     #[struct_ops_method] fn cong_avoid(sk: *mut sock, ack: u32, acked: u32) { /* grow */ }
//     #[struct_ops_method] fn undo_cwnd(sk: *mut sock) -> u32 { /* restore */ 0 }
// }
//
// Until this lands, register the C object with bpftool (see demo.sh). The
// conceptual model is identical: BPF fills a kernel vtable, the kernel calls in.
