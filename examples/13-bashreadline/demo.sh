#!/usr/bin/env bash
# examples/13-bashreadline/demo.sh — build, deploy, run bashreadline on the VM.
# Then open an INTERACTIVE bash on the target and type commands to see them.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/bashreadline"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building bashreadline (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in
  build) build ;;
  run|*)
    build
    IP="$("$LAB/vm-ip.sh" "$VM")"
    GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$IP" 'ip route | awk "/default/ {print \$3; exit}"')"
    c_info "OTLP -> http://$GW:4318"
    c_info "after it attaches, open another SSH session and type commands interactively:"
    c_info "    ssh -t fedora@$IP bash -i      # then type: echo hello, ls, etc."
    c_info "(if no events appear, readline may be in libreadline — re-run with"
    c_info "  READLINE_LIB=/usr/lib64/libreadline.so.8 on the target)"
    c_step "deploying to $VM (Ctrl-C to stop)"
    OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
    ;;
esac
