#!/usr/bin/env bash
# enable-bpf-lsm.sh <vm> — ensure the BPF LSM is active on a lab guest.
# Checks /sys/kernel/security/lsm; if "bpf" is missing, appends it to the
# kernel cmdline with grubby and reboots. On most Fedora 44 installs bpf is
# already in the list, so this is usually just a verification.
set -euo pipefail
VM="${1:-ebpf-target}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
IP="$("$SCRIPT_DIR/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"
LSM="$($SSH 'cat /sys/kernel/security/lsm')"
echo "current LSMs on $VM: $LSM"
if echo "$LSM" | tr ',' '\n' | grep -qx bpf; then
  echo "bpf LSM already active — nothing to do."
  exit 0
fi
echo "bpf not in the LSM list — adding it and rebooting $VM ..."
$SSH "sudo grubby --update-kernel=ALL --args=\"lsm=${LSM},bpf\" && sudo reboot" || true
echo "waiting for $VM to come back ..."
sleep 20
for _ in $(seq 1 30); do
  if $SSH 'cat /sys/kernel/security/lsm' 2>/dev/null | tr ',' '\n' | grep -qx bpf; then
    echo "bpf LSM now active."; exit 0
  fi
  sleep 5
done
echo "timed out waiting for $VM; check: ssh fedora@$IP 'cat /sys/kernel/security/lsm'" >&2
exit 1
