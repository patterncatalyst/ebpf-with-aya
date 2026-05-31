#!/usr/bin/env bash
# examples/60-offload/demo.sh — attach an XDP counter requesting HW offload first,
# and report which mode the lab NIC actually supports. XDP_PASS, so safe on a
# live interface. There is no offload-capable NIC in the KVM lab — expect DRV/SKB.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/offload"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building offload (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
IFACE="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$5; exit}"')"
c_info "target=$TIP iface=$IFACE  (asking for HW offload; expect fallback to DRV/SKB on virtio)"
c_step "deploying offload to $VM (XDP_PASS — safe on the live NIC)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$IFACE" || true
c_step "what mode is actually attached?"
$SSH "fedora@$TIP" "ip -d link show dev $IFACE | grep -iE 'xdp|prog' || echo '(no xdp line — already detached)'; sudo bpftool net show 2>/dev/null | head -8"
