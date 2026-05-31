#!/usr/bin/env bash
# examples/61-power/demo.sh — per-command on-CPU shares via sched_switch; multiply
# by RAPL package energy where available (bare metal) to estimate watts. In the
# KVM VM, RAPL/powercap is typically absent — the demo reports the shares instead.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/power"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building power (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_step "is RAPL/powercap available on the target?"
$SSH "fedora@$TIP" 'ls /sys/class/powercap/intel-rapl 2>/dev/null && echo "RAPL present" || echo "no RAPL in this VM (expected) — shares only; run on bare metal for watts"'
c_step "deploying power to $VM"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- || true
c_info "bare-metal cross-check: sudo turbostat --interval 1   |   perf stat -a -e power/energy-pkg/ sleep 1"
