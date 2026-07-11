#!/usr/bin/env bash
# lab-down.sh — gracefully shut down the lab guests so nothing is left running.
#   ./lab-down.sh            # ACPI shutdown of ebpf-target and ebpf-peer
#   FORCE=1 ./lab-down.sh    # hard power-off any guest that ignores the ACPI request
#
# Run this when you're done for the day. It does NOT delete anything — disks and
# snapshots survive; bring the lab back with ./lab-up.sh. The observability stack
# is a separate container (see the note at the end).
set -euo pipefail
export LIBVIRT_DEFAULT_URI="${LIBVIRT_DEFAULT_URI:-qemu:///system}"
TARGET="${TARGET_VM:-ebpf-target}"; PEER="${PEER_VM:-ebpf-peer}"

for VM in "$TARGET" "$PEER"; do
  virsh dominfo "$VM" >/dev/null 2>&1 || { echo "skip $VM (not defined)"; continue; }
  state="$(virsh domstate "$VM" 2>/dev/null || echo unknown)"
  if [ "$state" != "running" ]; then echo "$VM already $state"; continue; fi
  echo "shutting down $VM ..."
  virsh shutdown "$VM" >/dev/null 2>&1 || true
done

echo "waiting up to 60s for graceful shutdown ..."
for _ in $(seq 1 60); do
  running=0
  for VM in "$TARGET" "$PEER"; do
    [ "$(virsh domstate "$VM" 2>/dev/null || true)" = "running" ] && running=1
  done
  [ "$running" = 0 ] && break
  sleep 1
done

for VM in "$TARGET" "$PEER"; do
  [ "$(virsh domstate "$VM" 2>/dev/null || true)" = "running" ] || continue
  if [ "${FORCE:-0}" = "1" ]; then
    echo "forcing off $VM"; virsh destroy "$VM" >/dev/null 2>&1 || true
  else
    echo "WARNING: $VM still running (guest ignored ACPI). Re-run with FORCE=1 to power it off." >&2
  fi
done

virsh list --all
echo
echo "note: the observability stack (grafana/otel-lgtm) is a separate container."
echo "      stop it with:  podman rm -f ebpf-lgtm   (or: examples/03-observability-stack/demo.sh down)"
