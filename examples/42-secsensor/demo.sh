#!/usr/bin/env bash
# examples/42-secsensor/demo.sh — attach the security sensor to the TARGET and
# exercise each event type (exec, ptrace via strace, setuid) so the classified
# stream and the labelled counters move.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/secsensor"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building secsensor (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP OTLP=http://$GW:4318  (exec + ptrace + setuid telemetry)"
# exercise all three event types in a loop
$SSH "fedora@$TIP" "command -v strace >/dev/null || sudo dnf install -y strace >/dev/null 2>&1 || true; nohup bash -c 'while true; do id >/dev/null; (sleep 5 & SP=\$!; strace -p \$SP -e trace=none >/dev/null 2>&1 & sleep 1; kill \$SP 2>/dev/null); sudo -u nobody id >/dev/null 2>&1 || true; sleep 2; done' >/tmp/secsensor.log 2>&1 & echo exercising exec/ptrace/setuid"
c_step "deploying secsensor to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
