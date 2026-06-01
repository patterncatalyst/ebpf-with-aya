#!/usr/bin/env bash
# examples/66-bcc-tools/demo.sh — resolve and drive several BCC tools through the
# Python summarizer on the lab VM, then show a minimal BCC program.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }
case "${1:-run}" in build) echo "nothing to build (Python + bcc-tools)"; exit 0;; esac
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
scp -q -o StrictHostKeyChecking=accept-new bcc_runner.py hello_bcc.py "fedora@$TIP:/tmp/"
R="sudo python3 /tmp/bcc_runner.py"
c_step "the installed suite"; $SSH "fedora@$TIP" 'ls /usr/share/bcc/tools 2>/dev/null | head || echo "install bcc-tools"'
c_step "execsnoop → execs per command (8s; run things on the VM to fill it)"; $SSH "fedora@$TIP" "$R execsnoop --duration 8 || true"
c_step "tcpconnect → busiest destinations (8s)"; $SSH "fedora@$TIP" "$R tcpconnect --duration 8 || true"
c_step "biolatency → one 5s histogram (captured + printed)"; $SSH "fedora@$TIP" "$R biolatency 5 1 || true"
c_step "what a BCC tool is: inline C compiled at runtime"; $SSH "fedora@$TIP" 'timeout 5 sudo python3 /tmp/hello_bcc.py || echo "needs python3-bcc + clang + kernel headers"'
c_info "more: $R opensnoop · $R runqlat · $R profile · /usr/share/bcc/tools/funccount 'vfs_*'"
