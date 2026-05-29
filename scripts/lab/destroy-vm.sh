#!/usr/bin/env bash
# destroy-vm.sh — tear a guest all the way down, including its disks.
#   ./destroy-vm.sh ebpf-target
set -euo pipefail
VM="${1:?usage: destroy-vm.sh <vm-name>}"
virsh destroy "$VM" 2>/dev/null || true
virsh undefine "$VM" --remove-all-storage --nvram 2>/dev/null \
  || virsh undefine "$VM" 2>/dev/null || true
rm -f "$HOME/.cache/ebpf-with-aya/${VM}.qcow2" "$HOME/.cache/ebpf-with-aya/${VM}-seed.img"
echo "destroyed $VM"
