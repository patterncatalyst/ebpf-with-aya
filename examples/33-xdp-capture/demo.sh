#!/usr/bin/env bash
# examples/33-xdp-capture/demo.sh — build xdp-capture, deploy to the TARGET VM,
# attach to its primary interface, and open/close connections from the peer so
# you see SYN/FIN/RST capture lines (a live tcpdump-style connection feed).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/xdp-capture"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building xdp-capture (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
SSH="ssh -o StrictHostKeyChecking=accept-new"
TIFACE="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$5; exit}"')"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP iface=$TIFACE OTLP=http://$GW:4318  (captures TCP SYN/FIN/RST)"
# target listener for the peer to connect to (each curl = one SYN + teardown)
$SSH "fedora@$TIP" 'pkill -f "ncat -lk 8081" || true; nohup ncat -lk 8081 >/dev/null 2>&1 & echo target listening 8081'
if [ -n "$PIP" ]; then
  $SSH "fedora@$PIP" "nohup bash -c 'for i in \$(seq 1 600); do curl -s -o /dev/null --max-time 1 http://$TIP:8081/ || true; sleep 0.5; done' >/dev/null 2>&1 & echo opening connections from peer"
fi
c_step "deploying xdp-capture to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$TIFACE"
