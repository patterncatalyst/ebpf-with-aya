#!/usr/bin/env bash
# examples/24-memleak/demo.sh — build memleak, ship+compile the leaker on the VM,
# start it, then watch its libc allocations for outstanding (leaked) ones.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
source "$REPO_ROOT/scripts/lib/_demo-bg.sh"   # reap guest-side load-gens on exit
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/memleak"; SECS="${SECS:-15}"
LIBC="${LIBC:-/usr/lib64/libc.so.6}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building memleak (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"
GW="$($SSH 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "OTLP -> http://$GW:4318"
c_step "compiling + starting the leaker on $VM"
scp -o StrictHostKeyChecking=accept-new target-leaker/leaker.c "fedora@$IP:/home/fedora/leaker.c"
PID="$($SSH 'cd /home/fedora && clang -O0 -g -fno-omit-frame-pointer -o leaker leaker.c && pkill -x leaker || true; nohup ./leaker </dev/null >/tmp/leaker.log 2>&1 & echo $!')"
reap "fedora@$IP" leaker
c_ok "leaker pid $PID"
c_step "watching pid $PID for ${SECS}s, then reporting outstanding allocations"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$PID" "$LIBC" "$SECS"
