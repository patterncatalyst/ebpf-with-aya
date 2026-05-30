#!/usr/bin/env bash
# examples/30-sockops/demo.sh — build sockops, deploy to the TARGET VM, attach to
# the cgroup-v2 root, and open connections target↔peer to fire the callbacks.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/sockops"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building sockops (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
[ -z "$PIP" ] && c_fail "peer '$PEER' has no IP — provision it: $LAB/provision-vm.sh $PEER"
GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "target=$TIP peer=$PIP OTLP=http://$GW:4318  (sock_ops needs cgroup-v2 + privileges)"
ssh -o StrictHostKeyChecking=accept-new "fedora@$PIP" 'pkill -f "ncat -lk 9100" || true; nohup ncat -lk 9100 >/dev/null 2>&1 & echo peer listening'
ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" "pkill -f 'ncat -lk 9200' || true; nohup ncat -lk 9200 >/dev/null 2>&1 & echo target listening"
ssh -o StrictHostKeyChecking=accept-new "fedora@$TIP" "nohup bash -c 'for i in \$(seq 1 400); do curl -s -o /dev/null --max-time 1 http://$PIP:9100/ || true; sleep 0.4; done' >/dev/null 2>&1 & echo active connects"
ssh -o StrictHostKeyChecking=accept-new "fedora@$PIP" "nohup bash -c 'for i in \$(seq 1 400); do curl -s -o /dev/null --max-time 1 http://$TIP:9200/ || true; sleep 0.4; done' >/dev/null 2>&1 & echo passive connects"
c_step "deploying sockops to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
