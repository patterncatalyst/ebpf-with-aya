#!/usr/bin/env bash
# examples/55-structops-cc/demo.sh — compile a minimal BPF congestion-control
# algorithm and register it with bpftool struct_ops, the production path. Needs
# clang, libbpf-devel, and bpftool on the target (Chapter 4 toolchain).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }
case "${1:-run}" in build) echo "nothing to build on host (compiled on target)"; exit 0;; esac
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
$SSH "fedora@$TIP" 'mkdir -p /tmp/cc'
scp -q -o StrictHostKeyChecking=accept-new reference/cc.bpf.c "fedora@$TIP:/tmp/cc/cc.bpf.c"
c_step "generate vmlinux.h and compile the CC algorithm (target)"
$SSH "fedora@$TIP" 'cd /tmp/cc && sudo bpftool btf dump file /sys/kernel/btf/vmlinux format c > vmlinux.h && clang -O2 -g -Wall -target bpf -I. -c cc.bpf.c -o cc.o && echo compiled || echo "compile failed — need clang + libbpf-devel"'
c_step "register it as a struct_ops (pins a link to keep it active)"
$SSH "fedora@$TIP" 'cd /tmp/cc && sudo rm -rf /sys/fs/bpf/bpf_reno 2>/dev/null; sudo bpftool struct_ops register cc.o /sys/fs/bpf/bpf_reno 2>&1 && echo registered || echo "register failed — kernel may lack bpf struct_ops CC support"'
c_step "the kernel now offers it alongside cubic/reno"
$SSH "fedora@$TIP" 'sysctl net.ipv4.tcp_available_congestion_control; echo; sudo bpftool struct_ops show 2>/dev/null | grep -i reno || true'
c_info "select it with: sudo sysctl -w net.ipv4.tcp_congestion_control=bpf_reno   then: ss -ti"
c_step "cleanup (unregister)"
# struct_ops with a .struct_ops.link pins as a directory on this kernel, so -rf
$SSH "fedora@$TIP" 'sudo rm -rf /sys/fs/bpf/bpf_reno; echo unregistered'
