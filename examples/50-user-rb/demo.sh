#!/usr/bin/env bash
# examples/50-user-rb/demo.sh — produce a stream of samples from user space into
# a user ring buffer; a BPF program drains and aggregates them. Needs kernel >= 6.1.
# EXPERIMENTAL: Aya's user-ringbuf wrappers are still settling; see README.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/user-rb"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building user-rb (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
$SSH "fedora@$TIP" 'test "$(uname -r | cut -d. -f1)" -ge 6 || echo "WARNING: need kernel >= 6.1 for BPF_MAP_TYPE_USER_RINGBUF"'
c_info "target=$TIP OTLP=http://$GW:4318  (user space produces; the BPF program consumes)"
c_step "deploying user-rb to $VM"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
