#!/usr/bin/env bash
# examples/35-xdp-test/demo.sh — build the BPF_PROG_TEST_RUN harness, deploy it
# to the TARGET VM, and run it there under sudo. No peer VM and no live traffic:
# the program is exercised against synthetic packets and the verdicts asserted.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/xdp-test"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building xdp-test (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
c_info "runs the test harness once on $VM (under sudo); exits non-zero if any case fails"
c_step "deploying xdp-test to $VM"
"$LAB/deploy-to-target.sh" "$VM" "$BIN" --
