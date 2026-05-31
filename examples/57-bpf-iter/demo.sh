#!/usr/bin/env bash
# examples/57-bpf-iter/demo.sh — build a task iterator, pin it with bpftool, and
# cat a process table assembled entirely in the kernel. Needs clang +
# libbpf-devel + bpftool on the target (Chapter 4 toolchain).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }
case "${1:-run}" in build) echo "nothing to build on host (compiled on target)"; exit 0;; esac
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
$SSH "fedora@$TIP" 'mkdir -p /tmp/iter'
scp -q -o StrictHostKeyChecking=accept-new reference/task_iter.bpf.c "fedora@$TIP:/tmp/iter/"
c_step "generate vmlinux.h and compile the iterator (target)"
$SSH "fedora@$TIP" 'cd /tmp/iter && sudo bpftool btf dump file /sys/kernel/btf/vmlinux format c > vmlinux.h && clang -O2 -g -Wall -target bpf -I. -c task_iter.bpf.c -o task_iter.o && echo compiled || echo "compile failed — need clang + libbpf-devel"'
c_step "pin the iterator to bpffs"
$SSH "fedora@$TIP" 'cd /tmp/iter && sudo rm -f /sys/fs/bpf/task_iter; sudo bpftool iter pin task_iter.o /sys/fs/bpf/task_iter 2>&1 && echo pinned || echo "iter pin failed"'
c_step "cat it — a process table your BPF program built (first 15 rows)"
$SSH "fedora@$TIP" 'sudo cat /sys/fs/bpf/task_iter 2>/dev/null | head -15; echo ...; sudo cat /sys/fs/bpf/task_iter 2>/dev/null | tail -1'
c_info "every cat re-runs the iteration fresh; a map-element iterator adds 'map MAP'"
$SSH "fedora@$TIP" 'sudo rm -f /sys/fs/bpf/task_iter; echo cleaned up'
