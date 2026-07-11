#!/usr/bin/env bash
# examples/28-tcpstates/demo.sh — build tcpstates, deploy to the TARGET VM,
# then open + close TCP connections target→peer to exercise the state machine.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
source "$REPO_ROOT/scripts/lib/_demo-bg.sh"   # reap guest-side load-gens on exit
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/tcpstates"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building tcpstates (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
[ -z "$PIP" ] && c_fail "peer '$PEER' has no IP — provision it: $LAB/provision-vm.sh $PEER"
GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "target=$TIP  peer=$PIP  OTLP=http://$GW:4318"
ssh -o StrictHostKeyChecking=accept-new "fedora@$PIP" 'pkill -x ncat || true; nohup ncat -lk 8080 </dev/null >/dev/null 2>&1 & echo peer listening'
reap "fedora@$PIP" ncat
ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" "nohup bash -c 'for i in \$(seq 1 400); do curl -s -o /dev/null --max-time 1 http://$PIP:8080/ || true; sleep 0.4; done' </dev/null >/dev/null 2>&1 & echo opening/closing connections"
reap "fedora@$TIP" 'seq 1 400); do curl -s -o /dev/null --max-time 1'
c_step "deploying tcpstates to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
