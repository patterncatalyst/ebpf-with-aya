#!/usr/bin/env bash
# examples/38-signal-kill/demo.sh — deploy the kill-on-exec program to the
# TARGET, then loop: run a forbidden binary (/tmp/forbidden-sleep, gets killed)
# and a normal one (sleep, survives). LAB-ONLY.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
source "$REPO_ROOT/scripts/lib/_demo-bg.sh"   # reap guest-side load-gens on exit
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/signal-kill"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building signal-kill (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP OTLP=http://$GW:4318  (kills exec of /tmp/forbidden*)"
$SSH "fedora@$TIP" "cp /usr/bin/sleep /tmp/forbidden-sleep 2>/dev/null || true; nohup bash -c 'while true; do /tmp/forbidden-sleep 30 & wait \$! ; echo \"forbidden exit=\$?\"; sleep 60 & SP=\$!; kill \$SP 2>/dev/null; sleep 2; done' >/tmp/killer.log 2>&1 & echo running forbidden + normal binaries - tail /tmp/killer.log"
reap "fedora@$TIP" 'while true; do /tmp/forbidden-sleep'
c_step "deploying signal-kill to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
