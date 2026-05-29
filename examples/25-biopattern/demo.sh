#!/usr/bin/env bash
# examples/25-biopattern/demo.sh — build biopattern, deploy to VM, run it.
# Generate sequential then random disk I/O on the VM to see the ratio move.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/biopattern"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building biopattern (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$IP" 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "OTLP -> http://$GW:4318"
c_info "drive I/O on the VM (sequential write, then random reads):"
c_info "    ssh fedora@$IP 'dd if=/dev/zero of=/tmp/seq bs=1M count=512 oflag=direct; sync'"
c_info "    ssh fedora@$IP 'fio --name=rand --filename=/tmp/seq --rw=randread --bs=4k --size=256m --direct=1 --runtime=20 --time_based 2>/dev/null || echo \"(install fio for a clean random workload: sudo dnf install -y fio)\"'"
c_info "    map DEV numbers to disks:  ssh fedora@$IP 'lsblk; cat /proc/partitions'"
c_step "deploying biopattern to $VM (table every 2s; Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
