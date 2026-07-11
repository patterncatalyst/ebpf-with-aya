#!/usr/bin/env bash
# examples/43-scx-simple/demo.sh — run the real scx_simple scheduler on the
# TARGET (kernel >= 6.12), drive a CPU workload, and attach an Aya sched_switch
# probe to watch it schedule. Stopping scx_simple reverts to the default.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
source "$REPO_ROOT/scripts/lib/_demo-bg.sh"   # reap guest-side load-gens on exit
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/scx-watch"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building scx-watch (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
$SSH "fedora@$TIP" 'test -d /sys/kernel/sched_ext' || c_fail "sched_ext not available on $VM (need kernel >= 6.12 with CONFIG_SCHED_CLASS_EXT)"
c_info "target=$TIP OTLP=http://$GW:4318  (running scx_simple + an Aya sched_switch probe)"
$SSH "fedora@$TIP" 'command -v scx_simple >/dev/null || sudo dnf install -y scx-scheds >/dev/null 2>&1 || true; command -v scx_simple >/dev/null && echo scx_simple present || echo "scx_simple missing — install scx-scheds"'
# start scx_simple as the active scheduler, then a CPU workload
$SSH "fedora@$TIP" 'sudo pkill -x scx_simple 2>/dev/null || true; sudo nohup scx_simple >/tmp/scx_simple.log 2>&1 & sleep 2; echo "sched_ext state: $(cat /sys/kernel/sched_ext/state 2>/dev/null)"'
reap "fedora@$TIP" scx_simple
$SSH "fedora@$TIP" 'nohup bash -c "for i in \$(seq 1 8); do (timeout 60 yes >/dev/null &) ; done" >/dev/null 2>&1 & echo started CPU workload'
reap "fedora@$TIP" 'timeout 60 yes'
c_info "stop the scheduler later with: ssh fedora@$TIP 'sudo pkill -x scx_simple'  (reverts to default)"
c_step "deploying scx-watch probe to $VM (Ctrl-C to stop the probe)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
