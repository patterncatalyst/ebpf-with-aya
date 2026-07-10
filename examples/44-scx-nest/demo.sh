#!/usr/bin/env bash
# examples/44-scx-nest/demo.sh — run the real scx_nest on the TARGET, drive a
# MODERATE load (fewer busy tasks than cores), and attach an Aya per-CPU busy
# probe so the nest (a few hot cores, rest idle) becomes visible.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/cpu-busy"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building cpu-busy (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
$SSH "fedora@$TIP" 'test -d /sys/kernel/sched_ext' || c_fail "sched_ext not available on $VM (need kernel >= 6.12)"
c_info "target=$TIP OTLP=http://$GW:4318  (scx_nest + moderate load + per-CPU busy probe)"
$SSH "fedora@$TIP" 'command -v scx_nest >/dev/null || sudo dnf install -y scx-scheds >/dev/null 2>&1 || true; command -v scx_nest >/dev/null && echo scx_nest present || echo "scx_nest missing — install scx-scheds"'
$SSH "fedora@$TIP" 'sudo pkill -x scx_simple 2>/dev/null; sudo pkill -x scx_nest 2>/dev/null; sudo nohup scx_nest >/tmp/scx_nest.log 2>&1 & sleep 2; echo "sched_ext: $(cat /sys/kernel/sched_ext/root/ops 2>/dev/null) state=$(cat /sys/kernel/sched_ext/state 2>/dev/null)"'
# moderate load: 2 busy tasks (fewer than cores) — the regime where the nest shows
$SSH "fedora@$TIP" 'nohup bash -c "for i in 1 2; do (timeout 90 yes >/dev/null &) ; done" >/dev/null 2>&1 & echo started moderate load - 2 busy tasks'
c_info "compare on target: mpstat -P ALL 2 1   (a few CPUs hot, rest idle).  Stop: sudo pkill -x scx_nest"
c_step "deploying cpu-busy probe to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
