#!/usr/bin/env bash
# examples/68-he-observability/demo.sh
# Build the TFHE-rs workload AND the Aya observer, ship both to the VM, then
# attach the observer to the workload's he_* boundaries and time each operation.
#
# The workload is started a few seconds LATE on purpose: the observer attaches
# to the binary path first, so it also catches the one-shot he_keygen at startup.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
OBS="$SCRIPT_DIR/target/release/he-observer"
APP="$SCRIPT_DIR/target/release/he-workload"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){
  c_step "building he-observer + he-workload (release) — TFHE-rs build is heavy"
  cargo build --release || c_fail "cargo build failed"
  [[ -x "$OBS" && -x "$APP" ]] || c_fail "missing binaries in target/release"
  c_ok "built observer + workload"
}
case "${1:-run}" in
  build) build ;;
  run|*)
    build
    IP="$("$LAB/vm-ip.sh" "$VM")"
    SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"
    GW="$($SSH 'ip route | awk "/default/ {print \$3; exit}"')"
    c_info "OTLP -> http://$GW:4318"
    c_step "shipping he-workload to $VM (starts in 4s so the probe attaches first)"
    scp -o StrictHostKeyChecking=accept-new "$APP" "fedora@$IP:/home/fedora/he-workload"
    $SSH 'chmod +x /home/fedora/he-workload; pkill -f /home/fedora/he-workload || true; nohup sh -c "sleep 4; /home/fedora/he-workload" >/tmp/he-workload.log 2>&1 & echo scheduled'
    c_info "workload will log to /tmp/he-workload.log on the VM"
    c_step "deploying he-observer and attaching to he_* (Ctrl-C to stop)"
    OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$OBS" -- /home/fedora/he-workload
    ;;
esac
