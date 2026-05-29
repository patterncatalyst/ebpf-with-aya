#!/usr/bin/env bash
# examples/23-profile/demo.sh — build profile, deploy to VM, sample for N secs,
# capture folded stacks. Generate CPU load on the VM so there's something to see.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/profile"; SECS="${SECS:-10}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building profile (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$IP" 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "OTLP -> http://$GW:4318"
c_info "generate CPU load on the VM in another terminal while this samples:"
c_info "    ssh fedora@$IP 'timeout ${SECS} bash -c \"while :; do :; done\" & timeout ${SECS} sha256sum /dev/zero'"
c_step "deploying profile to $VM, sampling ${SECS}s (folded stacks to stdout)"
c_info "tip: redirect to a file:  ./demo.sh > out.folded   then  flamegraph.pl out.folded > cpu.svg"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$SECS"
