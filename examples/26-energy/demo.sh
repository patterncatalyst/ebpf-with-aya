#!/usr/bin/env bash
# examples/26-energy/demo.sh — build energy, deploy to VM, attribute power to
# processes by CPU-time share. RAPL is usually absent on VMs, so it models
# system power as ENERGY_TDP_WATTS (default 15W); on bare metal it uses RAPL.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/energy"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building energy (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$IP" 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "OTLP -> http://$GW:4318"
c_info "RAPL on the VM? (usually no):  ssh fedora@$IP 'ls /sys/class/powercap/intel-rapl:0/energy_uj 2>/dev/null || echo none'"
c_info "make some CPU consumers to attribute power to:"
c_info "    ssh fedora@$IP 'timeout 30 sha256sum /dev/zero & timeout 30 md5sum /dev/zero & timeout 30 bash -c \"while :; do :; done\"'"
c_step "deploying energy to $VM (power-by-process table every 2s; Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
