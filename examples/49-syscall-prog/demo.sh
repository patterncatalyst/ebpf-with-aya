#!/usr/bin/env bash
# examples/49-syscall-prog/demo.sh — reveal a real loader/syscall program by
# generating a LIGHT skeleton (bpftool gen skeleton -L) from a compiled BPF
# object, on the TARGET. Point BPF_OBJ at any built example's eBPF object;
# default is the pin-demo object from Chapter 48 (build that example first).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
DEF="$REPO_ROOT/examples/48-pin-demo/target/bpfel-unknown-none/release/pin-demo"
BPF_OBJ="${BPF_OBJ:-$DEF}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_info(){ echo -e "\033[1;33m  $*\033[0m"; }
c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
[ -f "$BPF_OBJ" ] || c_fail "no BPF object at $BPF_OBJ — build an example first (e.g. cd ../48-pin-demo && cargo build --release), or set BPF_OBJ=<path-to-eBPF-object>"
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
c_info "object: $BPF_OBJ  ->  $VM:$TIP"
scp -q -o StrictHostKeyChecking=accept-new "$BPF_OBJ" "fedora@$TIP:/tmp/prog.o"
c_step "the LIGHT skeleton embeds a generated BPF_PROG_TYPE_SYSCALL loader program + data"
$SSH "fedora@$TIP" 'sudo bpftool gen skeleton -L /tmp/prog.o 2>/dev/null | sed -n "1,48p" || echo "bpftool gen skeleton -L not available on this build"'
c_step "for contrast: the ordinary (full) skeleton embeds the whole ELF"
$SSH "fedora@$TIP" 'sudo bpftool gen skeleton /tmp/prog.o 2>/dev/null | sed -n "1,12p" || true'
c_info "see illustrative/loader_program.rs for what such a loader program does"
