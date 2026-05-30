#!/usr/bin/env bash
# examples/32-xdp-drop/demo.sh — build xdp-drop, deploy to the TARGET VM, attach
# the XDP filter to its primary interface, then ping the target from the peer:
# ICMP is dropped in the driver (pings time out) while TCP keeps working.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/xdp-drop"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building xdp-drop (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
SSH="ssh -o StrictHostKeyChecking=accept-new"
TIFACE="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$5; exit}"')"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP iface=$TIFACE OTLP=http://$GW:4318  (drops ICMP at XDP)"
if [ -n "$PIP" ]; then
  c_info "from the peer, ICMP to the target will stop once attached:"
  $SSH "fedora@$PIP" "nohup bash -c 'for i in \$(seq 1 600); do ping -c1 -W1 $TIP >/dev/null 2>&1 && echo \"ping ok\" || echo \"ping DROPPED\"; sleep 1; done' >/tmp/xdp-ping.log 2>&1 & echo pinging target; tail -f /tmp/xdp-ping.log" &
fi
c_step "deploying xdp-drop to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$TIFACE"
