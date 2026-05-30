#!/usr/bin/env bash
# examples/29-http-l7/demo.sh — build httpl7, deploy to the TARGET VM, attach the
# socket filter to its interface, and drive HTTP from target → peer.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/httpl7"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building httpl7 (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
[ -z "$PIP" ] && c_fail "peer '$PEER' has no IP — provision it: $LAB/provision-vm.sh $PEER"
GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" 'ip route | awk "/default/ {print \$3; exit}"')"
IFACE="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" 'ip -o route get 1.1.1.1 | awk "{for(i=1;i<=NF;i++) if(\$i==\"dev\"){print \$(i+1);exit}}"')"
c_info "target=$TIP iface=$IFACE  peer=$PIP  OTLP=http://$GW:4318"
c_step "starting an HTTP server on the peer ($PEER:8000)"
ssh -o StrictHostKeyChecking=accept-new "fedora@$PIP" 'pkill -f "python3 -m http.server" || true; nohup python3 -m http.server 8000 >/dev/null 2>&1 & echo serving'
c_step "driving HTTP target→peer in the background"
ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" "nohup bash -c 'for i in \$(seq 1 600); do curl -s -o /dev/null http://$PIP:8000/ ; curl -s -o /dev/null -X POST http://$PIP:8000/submit ; sleep 0.3; done' >/dev/null 2>&1 & echo driving"
c_step "deploying httpl7 to $VM on iface $IFACE (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$IFACE"
