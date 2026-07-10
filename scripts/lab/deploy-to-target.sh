#!/usr/bin/env bash
# deploy-to-target.sh — copy a compiled user-space binary to a guest and run it.
#
#   ./deploy-to-target.sh ebpf-target ./target/release/hello
#   ./deploy-to-target.sh ebpf-target ./target/release/hello -- --iface eth0
#
# Everything after `--` is passed to the binary on the guest, under sudo
# (eBPF loading needs CAP_BPF/CAP_SYS_ADMIN; the lab user has passwordless sudo).
set -euo pipefail
VM="${1:?usage: deploy-to-target.sh <vm-name> <local-binary> [-- args...]}"
BIN="${2:?need a local binary path}"
shift 2
[[ "${1:-}" == "--" ]] && shift || true

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
IP="$("$SCRIPT_DIR/vm-ip.sh" "$VM")"
SSH_OPTS="-o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=$HOME/.ssh/known_hosts"
REMOTE="/home/fedora/$(basename "$BIN")"

echo "→ copying $(basename "$BIN") to fedora@$IP:$REMOTE"
scp $SSH_OPTS "$BIN" "fedora@$IP:$REMOTE"
ssh $SSH_OPTS "fedora@$IP" "chmod +x '$REMOTE'"
echo "→ running on $VM (Ctrl-C to stop):"
# Demos set OTEL_ENDPOINT to the host stack (http://<gateway>:4318). sudo strips
# the environment, so forward it explicitly as OTEL_EXPORTER_OTLP_ENDPOINT via
# `env` — otherwise the guest binary falls back to its own localhost and no
# telemetry reaches the host stack.
RENV=""
[[ -n "${OTEL_ENDPOINT:-}" ]] && RENV="OTEL_EXPORTER_OTLP_ENDPOINT='$OTEL_ENDPOINT'"
exec ssh -t $SSH_OPTS "fedora@$IP" "sudo env $RENV '$REMOTE' $*"
