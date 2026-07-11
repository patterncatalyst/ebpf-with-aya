#!/usr/bin/env bash
# examples/63-capstone/demo.sh — one request through FastAPI → Quarkus, observed
# at every layer (spans/metrics/logs + the eBPF view), correlated on one trace_id.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/capstone"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building eBPF observer"; cargo build --release || c_fail "cargo build failed"; }
case "${1:-run}" in build) build; exit 0;; esac
build
c_step "bring up both services (podman-compose)"
command -v podman-compose >/dev/null || c_info "podman-compose not found — install it (pip install --user podman-compose)"
podman-compose up --build -d || c_info "compose failed — see quarkus-app/Containerfile (Quarkus maven build; first run downloads deps and is slow)"
c_step "deploy + start the eBPF observer on $VM"
TIP="$("$LAB/vm-ip.sh" "$VM" 2>/dev/null || echo '')"
if [[ -n "$TIP" ]]; then GW="$(ssh "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"; OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- || true; else c_info "(no VM; run ./target/release/capstone locally with sudo to observe host containers)"; fi
sleep 3
c_step "fire ONE traced request: curl → FastAPI /checkout → Quarkus /inventory"
TID="$(openssl rand -hex 16)"; SID="$(openssl rand -hex 8)"
RESP="$(curl -s -H "traceparent: 00-${TID}-${SID}-01" http://127.0.0.1:8000/checkout || true)"
echo "  response: $RESP"
c_info "trace_id = ${TID}"
c_info "Grafana 127.0.0.1:3000:  Explore → Tempo → ${TID}  (spans from BOTH services)"
c_info "   span → Logs (Loki) · graph app_requests_total exemplar → trace · graph ebpf_capstone_syscalls_total for the window"
c_info "cleanup: podman-compose down"
