#!/usr/bin/env bash
# examples/49-syscall-prog/demo.sh — reveal a real loader/syscall program by
# generating a LIGHT skeleton (bpftool gen skeleton -L) from a compiled BPF
# object, on the TARGET. By default it compiles a tiny libbpf-style C object
# (reference/skel_demo.bpf.c) on the target; set BPF_OBJ=<path> to point at your
# own *libbpf-compatible* object instead. (An aya object won't work — aya emits
# legacy 'maps'-section definitions that libbpf v1.0+ skeletons reject.)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }
c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
case "${1:-run}" in build) echo "nothing to build on host (compiled on target)"; exit 0;; esac
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"

if [ -n "${BPF_OBJ:-}" ]; then
  [ -f "$BPF_OBJ" ] || c_fail "no BPF object at BPF_OBJ=$BPF_OBJ"
  c_info "object: $BPF_OBJ  ->  $VM:$TIP"
  scp -q -o StrictHostKeyChecking=accept-new "$BPF_OBJ" "fedora@$TIP:/tmp/prog.o"
else
  c_info "compiling reference/skel_demo.bpf.c on $VM:$TIP (needs clang + libbpf-devel)"
  $SSH "fedora@$TIP" 'mkdir -p /tmp/skel'
  scp -q -o StrictHostKeyChecking=accept-new reference/skel_demo.bpf.c "fedora@$TIP:/tmp/skel/skel_demo.bpf.c"
  $SSH "fedora@$TIP" 'cd /tmp/skel && sudo bpftool btf dump file /sys/kernel/btf/vmlinux format c > vmlinux.h && clang -O2 -g -Wall -target bpf -I. -c skel_demo.bpf.c -o /tmp/prog.o && echo compiled || echo "compile failed — need clang + libbpf-devel"'
fi

c_step "the LIGHT skeleton embeds a generated BPF_PROG_TYPE_SYSCALL loader program + data"
$SSH "fedora@$TIP" 'sudo bpftool gen skeleton -L /tmp/prog.o 2>&1 | sed -n "1,48p"'
c_step "for contrast: the ordinary (full) skeleton embeds the whole ELF"
$SSH "fedora@$TIP" 'sudo bpftool gen skeleton /tmp/prog.o 2>&1 | sed -n "1,12p"'
c_info "see illustrative/loader_program.rs for what such a loader program does"
