#!/usr/bin/env bash
# examples/54-bpf-timer/demo.sh — in-kernel per-second aggregation via a self-
# rescheduling bpf_timer. The loader only READS the rate; the kernel computes it.
# Needs kernel >= 5.15.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/timer"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building timer (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP OTLP=http://$GW:4318  (kernel computes the per-second rate; loader just reads it)"
c_step "deploying timer to $VM"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
