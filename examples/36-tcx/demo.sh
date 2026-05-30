#!/usr/bin/env bash
# examples/36-tcx/demo.sh — build tcx, deploy to the TARGET VM, attach an
# ingress classifier via tcx (no clsact qdisc), and drive a little traffic so
# the per-protocol counts rise. Contrast with Chapter 31's clsact attach.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/tcx"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building tcx (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
SSH="ssh -o StrictHostKeyChecking=accept-new"
TIFACE="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$5; exit}"')"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP iface=$TIFACE OTLP=http://$GW:4318  (tcx ingress; needs kernel >= 6.6)"
if [ -n "$PIP" ]; then
  $SSH "fedora@$PIP" "nohup bash -c 'for i in \$(seq 1 600); do ping -c1 -W1 $TIP >/dev/null 2>&1; curl -s -o /dev/null --max-time 1 http://$TIP:22 || true; sleep 0.4; done' >/dev/null 2>&1 & echo driving traffic to target"
fi
c_info "while attached, on the target: 'sudo bpftool net show' lists tcx/ingress; 'tc filter show dev $TIFACE ingress' is EMPTY"
c_step "deploying tcx to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$TIFACE"
