#!/usr/bin/env bash
# examples/27-tcpconnlat/demo.sh — build tcpconnlat, deploy to the TARGET VM,
# then drive TCP connects from the target to the PEER VM and watch the latency.
# Requires both guests provisioned (scripts/lab/provision-vm.sh ebpf-peer).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/tcpconnlat"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building tcpconnlat (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
[ -z "$PIP" ] && c_fail "peer '$PEER' has no IP — provision it: $LAB/provision-vm.sh $PEER"
GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "target=$TIP  peer=$PIP  OTLP=http://$GW:4318"
c_step "starting a listener on the peer ($PEER:8080)"
ssh -o StrictHostKeyChecking=accept-new "fedora@$PIP" 'pkill -f "ncat -lk 8080" || true; nohup ncat -lk 8080 >/dev/null 2>&1 & echo listening'
c_step "driving connects target→peer in the background"
ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" "nohup bash -c 'for i in \$(seq 1 600); do curl -s -o /dev/null --max-time 1 http://$PIP:8080/ || true; sleep 0.3; done' >/dev/null 2>&1 & echo driving"
c_step "deploying tcpconnlat to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
