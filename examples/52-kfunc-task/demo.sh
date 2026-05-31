#!/usr/bin/env bash
# examples/52-kfunc-task/demo.sh — look up a task by pid from a BPF program with
# the bpf_task_from_pid / bpf_task_release kfunc pair; the verifier enforces the
# release. Needs a kernel whose BTF exports those kfuncs (recent Fedora 44 ok).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/kfunc-task"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building kfunc-task (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
$SSH "fedora@$TIP" 'sudo bpftool btf dump file /sys/kernel/btf/vmlinux 2>/dev/null | grep -q bpf_task_from_pid && echo "kfunc present ✓" || echo "WARNING: bpf_task_from_pid not found in this kernel BTF"'
c_info "target=$TIP OTLP=http://$GW:4318  (phase 1 = our pid -> found; phase 2 = bogus -> missing)"
c_step "deploying kfunc-task to $VM"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
