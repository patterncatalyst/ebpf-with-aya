#!/usr/bin/env bash
# examples/46-three-signals/demo.sh — run a Java and a Python HTTP service on
# the TARGET, drive load at both, and attach the socket probe that turns each
# request into a span + log + metric sharing one trace_id. Then explore the
# metric -> trace -> logs correlation in Grafana.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/httpwatch"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building httpwatch (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_info "target=$TIP OTLP=http://$GW:4318  (Java :8081, Python :8082, + socket probe)"
$SSH "fedora@$TIP" 'mkdir -p /tmp/three-signals/java /tmp/three-signals/python'
scp -q -o StrictHostKeyChecking=accept-new services/java/* "fedora@$TIP:/tmp/three-signals/java/"
scp -q -o StrictHostKeyChecking=accept-new services/python/* "fedora@$TIP:/tmp/three-signals/python/"
$SSH "fedora@$TIP" 'cd /tmp/three-signals/java && podman build -t ts-java . && podman rm -f ts-java 2>/dev/null; podman run -d --name ts-java -p 8081:8080 ts-java >/dev/null && echo java up'
$SSH "fedora@$TIP" 'cd /tmp/three-signals/python && podman build -t ts-python . && podman rm -f ts-python 2>/dev/null; podman run -d --name ts-python -p 8082:8080 ts-python >/dev/null && echo python up'
sleep 2
$SSH "fedora@$TIP" 'nohup bash -c "while true; do curl -s -o /dev/null http://127.0.0.1:8081/; curl -s -o /dev/null http://127.0.0.1:8082/; sleep 0.1; done" </dev/null >/dev/null 2>&1 & echo driving load at both services'
c_info "Grafana 127.0.0.1:3000 — metric: histogram_quantile(0.95, sum by (le,service) (rate(ebpf_http_server_duration_ms_bucket[1m]))); then Tempo spans + Loki logs by trace_id"
c_step "deploying httpwatch to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" --
