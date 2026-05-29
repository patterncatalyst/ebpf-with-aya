#!/usr/bin/env bash
# examples/11-execsnoop/demo.sh — build, deploy, run execsnoop on the target VM.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/execsnoop"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building execsnoop (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in
  build) build ;;
  run|*)
    build
    IP="$("$LAB/vm-ip.sh" "$VM")"
    GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$IP" 'ip route | awk "/default/ {print \$3; exit}"')"
    c_info "OTLP -> http://$GW:4318"
    c_info "generate execs on the target, e.g.:  ssh fedora@$IP 'ls -la /tmp; uname -a; id'"
    c_step "deploying to $VM (Ctrl-C to stop)"
    OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
    ;;
esac
