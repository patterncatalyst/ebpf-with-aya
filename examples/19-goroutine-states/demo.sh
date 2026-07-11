#!/usr/bin/env bash
# examples/19-goroutine-states/demo.sh — build the Go target + the tracer, ship
# the Go binary to the VM, run it, attach the casgstatus uprobe.
# Requires the Go toolchain on the host (Fedora: sudo dnf install -y golang).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
source "$REPO_ROOT/scripts/lib/_demo-bg.sh"   # reap guest-side load-gens on exit
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/goroutine"
GOBIN="$SCRIPT_DIR/target-go/target-go"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){
  c_step "building Go target (needs golang on host)"
  command -v go >/dev/null || c_fail "go not found — sudo dnf install -y golang"
  ( cd target-go && go build -o target-go . ) || c_fail "go build failed"
  c_step "building goroutine tracer (release)"
  cargo build --release || c_fail "cargo build failed"
  [[ -x "$BIN" && -x "$GOBIN" ]] || c_fail "missing binaries"
  c_ok "built tracer + Go target"
}
case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"
GW="$($SSH 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "OTLP -> http://$GW:4318"
c_step "shipping Go target to $VM and starting it"
scp -o StrictHostKeyChecking=accept-new "$GOBIN" "fedora@$IP:/home/fedora/target-go"
$SSH 'chmod +x /home/fedora/target-go; pkill -x target-go || true; nohup /home/fedora/target-go </dev/null >/tmp/target-go.log 2>&1 & echo started pid $!'
reap "fedora@$IP" target-go
c_info "confirm the symbol exists:  ssh fedora@$IP 'go version /home/fedora/target-go; nm /home/fedora/target-go | grep runtime.casgstatus'"
c_step "attaching casgstatus uprobe (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- /home/fedora/target-go
