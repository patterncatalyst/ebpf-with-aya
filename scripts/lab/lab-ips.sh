#!/usr/bin/env bash
# scripts/lab/lab-ips.sh — print the IPs of the two lab guests, for the
# networking chapters that drive traffic between them.
#   eval "$(scripts/lab/lab-ips.sh)"   # exports TARGET_IP and PEER_IP
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TARGET="${TARGET_VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"
ti="$("$SCRIPT_DIR/vm-ip.sh" "$TARGET" 2>/dev/null || true)"
pi="$("$SCRIPT_DIR/vm-ip.sh" "$PEER" 2>/dev/null || true)"
echo "export TARGET_IP=${ti}"
echo "export PEER_IP=${pi}"
[ -z "$pi" ] && echo "# peer '$PEER' has no IP — provision it: scripts/lab/provision-vm.sh $PEER" >&2 || true
