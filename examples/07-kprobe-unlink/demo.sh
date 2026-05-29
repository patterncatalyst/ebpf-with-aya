#!/usr/bin/env bash
#
# examples/07-kprobe-unlink/demo.sh
#
# Build unlinksnoop on the host, deploy to the target VM, run it, generate
# unlink traffic on the target so events flow, and point you at Grafana.
#
#   ./demo.sh                 # build + deploy + run (Ctrl-C to stop)
#   ./demo.sh build           # just cargo build --release on the host
#   VM=ebpf-target ./demo.sh  # override target VM name

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
BIN="$SCRIPT_DIR/target/release/unlinksnoop"

c_step() { echo -e "\033[0;36m━━ $*\033[0m"; }
c_ok()   { echo -e "\033[0;32m✓ $*\033[0m"; }
c_info() { echo -e "\033[1;33m  $*\033[0m"; }
c_fail() { echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }

build() {
  c_step "building unlinksnoop (release) on the host"
  cargo build --release || c_fail "cargo build failed — see errors above"
  [[ -x "$BIN" ]] || c_fail "expected binary not found at $BIN"
  c_ok "built $BIN"
}

case "${1:-run}" in
  build) build ;;
  run|*)
    build
    IP="$("$LAB/vm-ip.sh" "$VM")"
    GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$IP" 'ip route | awk "/default/ {print \$3; exit}"')"
    c_info "target exports OTLP to the host stack at http://$GW:4318"
    c_info "in another terminal, generate unlink traffic on the target:"
    c_info "    ssh fedora@$IP 'for i in \$(seq 1 20); do t=\$(mktemp); rm -f \"\$t\"; done'"
    c_step "deploying to $VM and running (Ctrl-C to stop)"
    OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
    ;;
esac
