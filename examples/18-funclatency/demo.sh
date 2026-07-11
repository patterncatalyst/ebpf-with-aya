#!/usr/bin/env bash
# examples/18-funclatency/demo.sh — build funclatency + target-app, ship the
# target to the VM, start it, then time slow_op() with uprobe/uretprobe.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
SNOOP="$SCRIPT_DIR/target/release/funclatency"
APP="$SCRIPT_DIR/target/release/target-app"
SYM="${SYM:-slow_op}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building funclatency + target-app (release, debug info)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$SNOOP" && -x "$APP" ]] || c_fail "missing binaries"; c_ok "built"; }
case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"
GW="$($SSH 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "OTLP -> http://$GW:4318"
c_step "shipping target-app to $VM and starting it"
scp -o StrictHostKeyChecking=accept-new "$APP" "fedora@$IP:/home/fedora/target-app"
$SSH 'chmod +x /home/fedora/target-app; pkill -x target-app || true; nohup /home/fedora/target-app </dev/null >/tmp/target-app.log 2>&1 & echo started pid $!'
c_step "timing $SYM (Ctrl-C to stop; prints a histogram every 2s)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$SNOOP" -- /home/fedora/target-app "$SYM"
