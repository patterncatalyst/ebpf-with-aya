#!/usr/bin/env bash
# examples/62-correlation/demo.sh — run an instrumented FastAPI service in Podman,
# hit it with a traceparent-carrying curl, and emit a span + metric + log on ONE
# trace_id. Then follow the metric → trace → logs path in Grafana. Runs on the
# host (where the Chapter 3 otel-lgtm stack and Podman live).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }
case "${1:-run}" in build) c_step "building image"; podman build -t ebpf-correlation-fastapi ./app; exit 0;; esac
c_step "build the instrumented FastAPI image"
podman build -t ebpf-correlation-fastapi ./app
c_step "run it (OTLP → host otel-lgtm at 4318)"
podman rm -f ebpf-correlation-fastapi 2>/dev/null || true
podman run -d --name ebpf-correlation-fastapi -p 8000:8000 \
  -e OTEL_EXPORTER_OTLP_ENDPOINT="http://host.containers.internal:4318" \
  ebpf-correlation-fastapi
sleep 3
c_step "hit /work with a traceparent — one trace across the span/metric/log"
TID="$(openssl rand -hex 16)"; SID="$(openssl rand -hex 8)"
curl -s -H "traceparent: 00-${TID}-${SID}-01" http://127.0.0.1:8000/work; echo
c_info "trace_id = ${TID}"
c_info "Grafana (127.0.0.1:3000): Explore → Tempo → paste ${TID} → open the span → click Logs (Loki)"
c_info "  then graph app_requests_total and click its exemplar dot to return to the trace"
c_step "confirm Tempo has it"
curl -s "http://127.0.0.1:3200/api/traces/${TID}" >/dev/null && echo "  trace present in Tempo" || echo "  (give it a few seconds, then re-query)"
c_info "cleanup: podman rm -f ebpf-correlation-fastapi"
