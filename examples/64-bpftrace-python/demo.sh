#!/usr/bin/env bash
# examples/64-bpftrace-python/demo.sh — drive a bpftrace program from Python on
# the lab VM (where bpftrace lives), rendering a live syscall-top table.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }
case "${1:-run}" in build) echo "nothing to build (Python + bpftrace)"; exit 0;; esac
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
c_step "copy the wrapper + programs to $VM"
$SSH "fedora@$TIP" 'rm -rf /tmp/bptool && mkdir -p /tmp/bptool/programs'
scp -q -o StrictHostKeyChecking=accept-new bpftrace_tool.py "fedora@$TIP:/tmp/bptool/"
scp -q -o StrictHostKeyChecking=accept-new programs/*.bt "fedora@$TIP:/tmp/bptool/programs/"
c_step "syscall top (counts per command, 8s)"
$SSH "fedora@$TIP" 'sudo python3 /tmp/bptool/bpftrace_tool.py --program /tmp/bptool/programs/syscount.bt --duration 8 || echo "needs bpftrace + python3 on the VM"'
c_info "try the read-size histogram: sudo python3 /tmp/bptool/bpftrace_tool.py --program /tmp/bptool/programs/readsize.bt"
