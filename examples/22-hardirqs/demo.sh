#!/usr/bin/env bash
# examples/22-hardirqs/demo.sh — build hardirqs, deploy to VM, run it. Generate
# IRQ activity (network + disk) so per-IRQ totals are interesting.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/hardirqs"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building hardirqs (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$IP" 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "OTLP -> http://$GW:4318"
c_info "drive IRQ activity on the VM (network + disk) in another terminal:"
c_info "    ssh fedora@$IP 'ping -f -c 5000 $GW >/dev/null 2>&1; dd if=/dev/zero of=/tmp/f bs=1M count=512 oflag=direct 2>/dev/null; sync; rm -f /tmp/f'"
c_step "deploying hardirqs to $VM (table every 2s; Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
