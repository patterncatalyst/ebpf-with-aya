#!/usr/bin/env python3
"""What a BCC tool is underneath: an inline C BPF program that BCC compiles at
runtime (Clang/LLVM) and attaches from Python. Contrast with Aya, which compiles
Rust ahead of time into one binary. Needs python3-bcc + clang + kernel headers.
Run with sudo; Ctrl-C to stop. View output via: sudo cat /sys/kernel/debug/tracing/trace_pipe"""
from bcc import BPF

program = r'''
int hello(void *ctx) {
    bpf_trace_printk("clone() called\n");
    return 0;
}
'''

b = BPF(text=program)                                  # <-- compiled right here
b.attach_kprobe(event=b.get_syscall_fnname("clone"), fn_name="hello")
print("tracing clone() — run a command in another shell; Ctrl-C to stop")
try:
    b.trace_print()
except KeyboardInterrupt:
    pass
