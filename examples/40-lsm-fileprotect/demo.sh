#!/usr/bin/env bash
# examples/40-lsm-fileprotect/demo.sh — protect /tmp/ebpf-protected with BPF
# LSM, then loop: read it (works) and try to append (denied, even via sudo).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/lsm-fileprotect"
FILE="/tmp/ebpf-protected"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building lsm-fileprotect (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
c_step "ensuring BPF LSM is enabled on $VM"
"$LAB/enable-bpf-lsm.sh" "$VM" || c_info "could not auto-enable; check: cat /sys/kernel/security/lsm"
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
$SSH "fedora@$TIP" "echo 'important config — do not tamper' > $FILE"
c_info "target=$TIP protecting=$FILE OTLP=http://$GW:4318"
# background: read (ok) + append (should be denied)
$SSH "fedora@$TIP" "nohup bash -c 'while true; do cat $FILE >/dev/null 2>&1 && echo READ-OK; (echo tamper >> $FILE) 2>/dev/null && echo WRITE-OK || echo WRITE-DENIED; sleep 1; done' >/tmp/fileprotect.log 2>&1 & echo driving read/write - tail /tmp/fileprotect.log"
c_step "deploying lsm-fileprotect to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$FILE"
