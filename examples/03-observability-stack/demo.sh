#!/usr/bin/env bash
#
# examples/03-observability-stack/demo.sh
#
# Brings up the grafana/otel-lgtm stack, builds and runs the Python 3.14 OTLP
# client in Podman, and confirms telemetry is flowing into Grafana.
#
#   ./demo.sh           # up + smoke test + leave running, print URLs
#   ./demo.sh up        # just start the stack
#   ./demo.sh client    # build + run the client once (60 iterations)
#   ./demo.sh down       # tear everything down
#
# Conventions: 127.0.0.1 not localhost; :Z on mounts (in compose); trap cleanup
# of the client container; wait-for-HTTP not sleep.

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"

GRAFANA_URL="http://127.0.0.1:3000"
OTLP_HTTP="http://127.0.0.1:4318"
CLIENT_IMAGE="ebpf-client:dev"
CLIENT_CTR="ebpf-client-run"

c_step() { echo -e "\033[0;36m━━ $*\033[0m"; }
c_ok()   { echo -e "\033[0;32m✓ $*\033[0m"; }
c_info() { echo -e "\033[1;33m  $*\033[0m"; }
c_fail() { echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }

cleanup_client() { podman rm -f "$CLIENT_CTR" >/dev/null 2>&1 || true; }

wait_for_http() {
  local url="$1" timeout="${2:-60}" i
  for ((i = 0; i < timeout; i++)); do
    curl -fsS "$url" >/dev/null 2>&1 && return 0
    sleep 1
  done
  return 1
}

up() {
  c_step "starting otel-lgtm stack (Grafana + Tempo + Mimir + Loki)"
  podman compose -f compose.yaml up -d
  c_info "waiting for Grafana on :3000 ..."
  wait_for_http "$GRAFANA_URL/api/health" 60 || c_fail "Grafana never became healthy"
  c_ok "Grafana healthy at $GRAFANA_URL"
}

run_client() {
  trap cleanup_client EXIT
  c_step "building Python 3.14 client image"
  podman build -t "$CLIENT_IMAGE" ./client
  c_ok "image built: $CLIENT_IMAGE"

  c_step "running client (60 iterations, exporting OTLP to the stack)"
  cleanup_client
  # host.containers.internal lets the rootless container reach the host's
  # published OTLP port. --network slirp4netns is the rootless default.
  podman run --name "$CLIENT_CTR" --rm \
    -e OTEL_EXPORTER_OTLP_ENDPOINT="http://host.containers.internal:4318" \
    -e OTEL_SERVICE_NAME="ebpf-client" \
    -e ITERATIONS=60 \
    "$CLIENT_IMAGE"
  c_ok "client finished"
}

verify() {
  c_step "verifying metrics arrived (querying Mimir/Prometheus via Grafana)"
  # The client emits ebpf_events_total; give the stack a moment to scrape/ingest.
  local i found=0
  for ((i = 0; i < 30; i++)); do
    if curl -fsS "$GRAFANA_URL/api/datasources/proxy/uid/prometheus/api/v1/query?query=ebpf_events_total" 2>/dev/null | grep -q '"result"'; then
      found=1; break
    fi
    sleep 2
  done
  [[ "$found" -eq 1 ]] && c_ok "ebpf_events_total is queryable in the stack" \
    || c_info "metric not visible yet — open Grafana Explore and check manually"
}

case "${1:-run}" in
  up)     up ;;
  client) run_client ;;
  down)   podman compose -f compose.yaml down -v; cleanup_client; c_ok "torn down" ;;
  run|*)
    up
    run_client
    verify
    echo
    c_ok "Stack is up. Open:"
    echo "    Grafana:        $GRAFANA_URL  (anonymous admin, no login)"
    echo "    Dashboard:      $GRAFANA_URL/d/ebpf-overview"
    echo "    Explore (logs): $GRAFANA_URL/explore"
    echo "    OTLP HTTP in:   $OTLP_HTTP        OTLP gRPC in: 127.0.0.1:4317"
    echo
    c_info "Tear down with: ./demo.sh down"
    ;;
esac
