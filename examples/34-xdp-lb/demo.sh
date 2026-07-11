#!/usr/bin/env bash
# examples/34-xdp-lb/demo.sh — build xdp-lb, start 3 UDP backend listeners on
# the TARGET VM, attach the balancer to its interface, then fire UDP datagrams
# from the peer at VIP:8080 and watch them fan out across 9001/9002/9003.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/xdp-lb"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building xdp-lb (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
[ -z "$PIP" ] && c_fail "peer '$PEER' has no IP — provision it: $LAB/provision-vm.sh $PEER"
SSH="ssh -o StrictHostKeyChecking=accept-new"
TIFACE="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$5; exit}"')"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP iface=$TIFACE peer=$PIP OTLP=http://$GW:4318  (VIP udp:8080 → 9001/9002/9003)"
# three UDP backend listeners on the target, each tagged so you can see the split
$SSH "fedora@$TIP" 'pkill -x ncat 2>/dev/null || true; for p in 9001 9002 9003; do nohup ncat -u -lk $p </dev/null >/tmp/backend-$p.log 2>&1 & done; echo backends listening'
# fire UDP datagrams from the peer at the VIP
$SSH "fedora@$PIP" "nohup bash -c 'for i in \$(seq 1 600); do echo req-\$i | ncat -u -w1 $TIP 8080; sleep 0.3; done' </dev/null >/dev/null 2>&1 & echo sending UDP to VIP:8080"
c_info "watch the split on the target: ssh fedora@$TIP 'tail -f /tmp/backend-90*.log'"
c_step "deploying xdp-lb to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$TIFACE"
