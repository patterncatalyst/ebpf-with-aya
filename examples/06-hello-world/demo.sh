#!/usr/bin/env bash
#
# examples/06-hello-world/demo.sh
#
# Build the hello-world Aya program on the host, deploy it to the target VM,
# run it there, generate execve traffic so the counter moves, and point you at
# Grafana. This is the canonical build->deploy->observe loop every later
# chapter reuses.
#
#   ./demo.sh                 # build + deploy + run (Ctrl-C to stop)
#   ./demo.sh build           # just cargo build --release on the host
#   VM=ebpf-target ./demo.sh  # override target VM name
#
# Requires: Chapter 4 toolchain on the host; Chapter 2 target VM up; the
# Chapter 3 stack running. Lab scripts are referenced from ../../scripts/lab.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
BIN="$SCRIPT_DIR/target/release/hello"

c_step() { echo -e "\033[0;36m━━ $*\033[0m"; }
c_ok()   { echo -e "\033[0;32m✓ $*\033[0m"; }
c_info() { echo -e "\033[1;33m  $*\033[0m"; }
c_fail() { echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }

build() {
  c_step "building hello (release) on the host"
  cargo build --release || c_fail "cargo build failed — see errors above (toolchain? bpf-linker?)"
  [[ -x "$BIN" ]] || c_fail "expected binary not found at $BIN"
  c_ok "built $BIN"
}

case "${1:-run}" in
  build) build ;;
  run|*)
    build
    # The stack runs on the host; from inside the VM it's the libvirt gateway.
    GW="$(ssh -o StrictHostKeyChecking=accept-new "fedora@$("$LAB/vm-ip.sh" "$VM")" 'ip route | awk "/default/ {print \$3; exit}"')"
    c_info "target will export OTLP to the host stack at http://$GW:4318"
    c_step "deploying to $VM and running (generates execve traffic to watch)"
    c_info "in another terminal you can run:  ssh fedora@\$($LAB/vm-ip.sh $VM) 'for i in {1..20}; do /bin/true; done'"
    OTEL_ENDPOINT="http://$GW:4318" \
      "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- # binary inherits OTEL_EXPORTER_OTLP_ENDPOINT via env on the guest
    ;;
esac
