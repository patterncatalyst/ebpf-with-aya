#!/usr/bin/env bash
# examples/39-pidhide/demo.sh — LAB-ONLY. Start a sleep on the TARGET, hide its
# PID from /proc, and loop ps/ls /proc to show it vanish (while it keeps
# running). Detach (Ctrl-C) and it reappears.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/pidhide"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building pidhide (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
# start a victim process and capture its pid
HPID="$($SSH "fedora@$TIP" 'nohup sleep 99999 >/dev/null 2>&1 & echo $!')"
c_info "target=$TIP hiding pid=$HPID OTLP=http://$GW:4318  (LAB-ONLY — taints the kernel)"
# loop showing whether the pid is visible
$SSH "fedora@$TIP" "nohup bash -c 'while true; do if ls /proc/$HPID >/dev/null 2>&1 && ps -p $HPID >/dev/null 2>&1; then echo VISIBLE; else echo HIDDEN kill-0:\$(kill -0 $HPID 2>/dev/null && echo alive || echo gone); fi; sleep 1; done' >/tmp/pidhide.log 2>&1 & echo watching pid $HPID - tail /tmp/pidhide.log"
c_step "deploying pidhide to $VM (Ctrl-C to stop, then the pid reappears)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$HPID"
