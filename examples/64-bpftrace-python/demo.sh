#!/usr/bin/env bash
# examples/64-bpftrace-python/demo.sh — drive several bpftrace programs from
# Python on the lab VM (where bpftrace lives), covering counts, streams, and a
# latency histogram.
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
R="sudo python3 /tmp/bptool/bpftrace_tool.py"
c_step "the bundled programs"
$SSH "fedora@$TIP" "$R --list"
c_step "1) syscall top (counts per command, 6s)"
$SSH "fedora@$TIP" "$R --program /tmp/bptool/programs/syscount.bt --duration 6 || true"
c_step "2) execsnoop (stream new processes, 6s — run something on the VM to see rows)"
$SSH "fedora@$TIP" "$R --program /tmp/bptool/programs/execsnoop.bt --duration 6 || true"
c_step "3) runqlat (scheduler-latency histogram, ~6s)"
$SSH "fedora@$TIP" "$R --program /tmp/bptool/programs/runqlat.bt --duration 6 || true"
c_info "more: opensnoop.bt killsnoop.bt profile.bt vfsstat.bt tcpconnect.bt readsize.bt"
c_info "inline: $R -e 'tracepoint:syscalls:sys_enter_openat { @[comm]=count(); } interval:s:1 { print(@); clear(@); }'"
