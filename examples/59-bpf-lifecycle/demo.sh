#!/usr/bin/env bash
# examples/59-bpf-lifecycle/demo.sh — pin a program's link + map so it outlives
# the loader; prove the program keeps running and its state survives a loader
# restart (lifetime decoupling + state continuity).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/lifecycle"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building lifecycle (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_step "run 1: load, PIN link + map, count, then exit (pins remain)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- || true
c_step "loader has exited — is the program still counting? (pinned link, no owner)"
$SSH "fedora@$TIP" 'sudo bpftool link show | tail -3; echo; echo "EVENTS now:"; sudo bpftool map dump pinned /sys/fs/bpf/ebpf-aya/EVENTS 2>/dev/null; sleep 1; echo "EVENTS 1s later (should be higher):"; sudo bpftool map dump pinned /sys/fs/bpf/ebpf-aya/EVENTS 2>/dev/null' || true
c_step "run 2: reuse the pinned map — the count CONTINUES, not resets"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- || true
c_info "cleanup (unpin → detaches): sudo rm -rf /sys/fs/bpf/ebpf-aya"
$SSH "fedora@$TIP" 'sudo rm -rf /sys/fs/bpf/ebpf-aya; echo unpinned'
