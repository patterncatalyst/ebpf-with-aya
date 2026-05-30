#!/usr/bin/env bash
# examples/31-tc-classify/demo.sh — build tc-classify, deploy to the TARGET VM,
# attach a tc egress classifier to its primary interface, then drive traffic to
# the peer: normal ports (passed + counted) and BLOCK_PORT 9999 (dropped).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/tc-classify"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building tc-classify (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
[ -z "$PIP" ] && c_fail "peer '$PEER' has no IP — provision it: $LAB/provision-vm.sh $PEER"
SSH="ssh -o StrictHostKeyChecking=accept-new"
TIFACE="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$5; exit}"')"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP iface=$TIFACE peer=$PIP OTLP=http://$GW:4318  (drops egress to :9999)"
# peer listeners: a normal port (passes) and the blocked port (egress-dropped on target)
$SSH "fedora@$PIP" 'pkill -f "ncat -lk 9100" || true; pkill -f "ncat -lk 9999" || true; nohup ncat -lk 9100 >/dev/null 2>&1 & nohup ncat -lk 9999 >/dev/null 2>&1 & echo peer listening 9100/9999'
# drive traffic from the target: passes to :9100, dropped to :9999 (will time out)
$SSH "fedora@$TIP" "nohup bash -c 'for i in \$(seq 1 600); do curl -s -o /dev/null --max-time 1 http://$PIP:9100/ || true; curl -s -o /dev/null --max-time 1 http://$PIP:9999/ || true; sleep 0.4; done' >/dev/null 2>&1 & echo driving traffic"
c_step "deploying tc-classify to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$TIFACE"
