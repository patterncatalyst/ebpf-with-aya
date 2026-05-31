#!/usr/bin/env bash
# examples/45-nginx-probe/demo.sh — build a symbol-keeping UBI nginx on the
# TARGET, run it, drive HTTP load, find the worker, and attach uprobes to
# measure per-request latency from inside nginx.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/nginx-probe"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building nginx-probe (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP OTLP=http://$GW:4318  (building nginx with symbols — first run is slow)"
$SSH "fedora@$TIP" 'mkdir -p /tmp/nginx-probe'
scp -o StrictHostKeyChecking=accept-new "$SCRIPT_DIR/Containerfile" "fedora@$TIP:/tmp/nginx-probe/Containerfile" >/dev/null
$SSH "fedora@$TIP" 'cd /tmp/nginx-probe && podman build -t ebpf-nginx . && podman rm -f ebpf-nginx 2>/dev/null; podman run -d --name ebpf-nginx -p 8080:80 ebpf-nginx && sleep 2 && echo nginx up on :8080'
# drive load
$SSH "fedora@$TIP" 'nohup bash -c "while true; do curl -s -o /dev/null http://127.0.0.1:8080/; sleep 0.05; done" >/dev/null 2>&1 & echo driving HTTP load'
sleep 1
WPID="$($SSH "fedora@$TIP" "pgrep -f 'nginx: worker' | head -1")"
[ -n "$WPID" ] || c_fail "could not find nginx worker pid on $VM"
TARGET="/proc/$WPID/root/usr/sbin/nginx"
c_info "worker pid=$WPID  nginx binary=$TARGET"
$SSH "fedora@$TIP" "nm $TARGET 2>/dev/null | grep -q ngx_http_process_request && echo 'symbols present ✓' || echo 'WARNING: symbols not found — install debuginfo or check the build'"
c_step "deploying nginx-probe to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$TARGET" "$WPID"
