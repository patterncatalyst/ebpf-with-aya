#!/usr/bin/env bash
# examples/37-lsm-confine/demo.sh — ensure BPF LSM is on, create a confined
# cgroup on the TARGET, confine it, and loop two curls: one from inside the
# confined cgroup (blocked with EPERM) and one from a normal shell (allowed).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"; BIN="$SCRIPT_DIR/target/release/lsm-confine"
CG="/sys/fs/cgroup/confined"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building lsm-confine (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
c_step "ensuring BPF LSM is enabled on $VM"
"$LAB/enable-bpf-lsm.sh" "$VM" || c_info "could not auto-enable; check: cat /sys/kernel/security/lsm"
TIP="$("$LAB/vm-ip.sh" "$VM")"
PIP="$("$LAB/vm-ip.sh" "$PEER" 2>/dev/null || true)"
DEST="${PIP:-93.184.216.34}"   # peer if present, else a public IP for the connect attempt
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP confined-cgroup=$CG dest=$DEST OTLP=http://$GW:4318"
$SSH "fedora@$TIP" "sudo mkdir -p $CG"
# background: confined curl (should be BLOCKED) vs normal curl (OK)
$SSH "fedora@$TIP" "nohup bash -c 'while true; do sudo bash -c \"echo \\\$\\\$ > $CG/cgroup.procs; curl -m2 -s -o /dev/null http://$DEST && echo CONFINED-OK || echo CONFINED-BLOCKED\"; curl -m2 -s -o /dev/null http://$DEST && echo HOST-OK || echo HOST-FAIL; sleep 1; done' >/tmp/confine.log 2>&1 & echo driving curls (tail /tmp/confine.log)"
c_step "deploying lsm-confine to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$CG"
