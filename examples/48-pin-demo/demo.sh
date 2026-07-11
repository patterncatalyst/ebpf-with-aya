#!/usr/bin/env bash
# examples/48-pin-demo/demo.sh — pin a program+map to bpffs so they outlive the
# loader: load (and exit), read the counter back from a fresh process, detach.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/pinctl"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building pinctl (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
scp -q -o StrictHostKeyChecking=accept-new "$BIN" "fedora@$TIP:/tmp/pinctl"
c_step "load + pin (the loader exits immediately after)"
$SSH "fedora@$TIP" 'sudo mount | grep -q "type bpf" || sudo mount -t bpf bpf /sys/fs/bpf; sudo /tmp/pinctl load'
c_step "loader is gone — but the objects persist in bpffs"
$SSH "fedora@$TIP" 'sudo ls -l /sys/fs/bpf/ebpf-aya/ ; echo; sudo bpftool prog show | grep -A2 tracepoint | head; echo; sudo bpftool link show | head'
c_step "read the counter from a fresh process (twice — watch it climb)"
$SSH "fedora@$TIP" "sudo OTEL_EXPORTER_OTLP_ENDPOINT=http://$GW:4318 /tmp/pinctl read"
sleep 2
$SSH "fedora@$TIP" "sudo OTEL_EXPORTER_OTLP_ENDPOINT=http://$GW:4318 /tmp/pinctl read"
c_step "detach (remove the pins -> program is freed)"
$SSH "fedora@$TIP" 'sudo /tmp/pinctl detach; echo; ls /sys/fs/bpf/ebpf-aya/ 2>/dev/null || echo "(pins gone)"'
c_ok "done — pinned, read without a loader, then detached"
