#!/usr/bin/env bash
# examples/53-bpf-token/demo.sh — the PRIVILEGED HALF you can see plainly: mount
# a bpffs with a tight delegation policy and print it. That mount is exactly the
# policy a container would derive a BPF token from. Needs kernel >= 6.9.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }
case "${1:-run}" in build) echo "nothing to build (control-plane demo)"; exit 0;; esac
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
MNT="/tmp/bpf-delegated"
OPTS="delegate_cmds=prog_load:map_create,delegate_progs=socket_filter,delegate_maps=ringbuf"
c_step "kernel version (need >= 6.9 for BPF token)"
$SSH "fedora@$TIP" 'uname -r'
c_step "mount a bpffs with a tight delegation policy"
$SSH "fedora@$TIP" "sudo mkdir -p $MNT && sudo mount -t bpf -o $OPTS bpffs $MNT 2>&1 && echo 'mounted with delegation' || echo 'mount failed — kernel may be < 6.9 or lack delegate_* support'"
c_step "the delegation policy the kernel will enforce (the four axes)"
$SSH "fedora@$TIP" "mount | grep -E 'bpf-delegated|delegate_' || true"
c_step "token support in this kernel"
$SSH "fedora@$TIP" 'sudo bpftool feature probe 2>/dev/null | grep -i token || echo "(no token line reported)"'
c_info "a container would now open $MNT and call bpf(BPF_TOKEN_CREATE) to derive a token,"
c_info "then pass its fd to bpf(PROG_LOAD/MAP_CREATE) — see illustrative/loader_with_token.rs"
$SSH "fedora@$TIP" "sudo umount $MNT 2>/dev/null; sudo rmdir $MNT 2>/dev/null || true; echo cleaned up"
