#!/usr/bin/env bash
# examples/65-bpftool-python/demo.sh — inventory/audit BPF on the lab VM. Starts a
# throwaway bpftrace probe so there's a loaded program to inspect, runs several
# wrapper commands, then cleans up.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
source "$REPO_ROOT/scripts/lib/_demo-bg.sh"   # reap guest-side load-gens on exit
VM="${VM:-ebpf-target}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }
case "${1:-run}" in build) echo "nothing to build (Python + bpftool)"; exit 0;; esac
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
scp -q -o StrictHostKeyChecking=accept-new bpftool_tool.py "fedora@$TIP:/tmp/bpftool_tool.py"
T="sudo python3 /tmp/bpftool_tool.py"
c_step "load a throwaway probe so there's something to inventory"
$SSH "fedora@$TIP" 'sudo sysctl -w kernel.bpf_stats_enabled=1 >/dev/null; (sudo bpftrace -e "kprobe:vfs_read { @[comm]=count(); }" </dev/null >/tmp/bt.log 2>&1 &) ; sleep 2; echo started'
reap "fedora@$TIP" bpftrace
c_step "progs — the host BPF inventory"; $SSH "fedora@$TIP" "$T progs || true"
c_step "maps"; $SSH "fedora@$TIP" "$T maps || true"
c_step "links"; $SSH "fedora@$TIP" "$T links || true"
c_step "audit — programs, holders, attachments"; $SSH "fedora@$TIP" "$T audit || true"
c_step "top — by avg ns/run (stats enabled)"; $SSH "fedora@$TIP" "$T top --enable-stats || true"
c_step "features — supported program/map types (truncated)"; $SSH "fedora@$TIP" "$T features 2>/dev/null | head -4 || true"
c_info "dump a map by name/id:  $T dump <name|id>"
$SSH "fedora@$TIP" 'sudo pkill -x bpftrace 2>/dev/null; echo cleaned up'
