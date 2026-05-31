#!/usr/bin/env bash
# examples/58-core/demo.sh — load the Aya CO-RE reader (relocations resolve
# against THIS kernel's BTF), and show the CO-RE relocation records in the
# canonical C object. Needs CONFIG_DEBUG_INFO_BTF; clang+libbpf-devel+llvm for
# the relocation-inspection step.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/core"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building core (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"; SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
c_step "the target must expose BTF for relocations to resolve"
$SSH "fedora@$TIP" 'ls -l /sys/kernel/btf/vmlinux && echo "BTF present" || echo "NO BTF — needs CONFIG_DEBUG_INFO_BTF"'
c_info "remember: aya-tool generate task_struct > core-ebpf/src/vmlinux.rs for real CO-RE bindings"
c_step "deploying the CO-RE reader to $VM"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- || true
c_step "show CO-RE relocation records in the canonical C object"
$SSH "fedora@$TIP" 'mkdir -p /tmp/core'
scp -q -o StrictHostKeyChecking=accept-new reference/core.bpf.c "fedora@$TIP:/tmp/core/"
$SSH "fedora@$TIP" 'cd /tmp/core && sudo bpftool btf dump file /sys/kernel/btf/vmlinux format c > vmlinux.h && clang -O2 -g -target bpf -I. -c core.bpf.c -o core.o 2>&1 && (llvm-objdump -r core.o 2>/dev/null | grep -i core || bpftool btf dump file core.o 2>/dev/null | grep -i reloc || echo "(inspect with: bpftool gen object / llvm-objdump -r)") || echo "compile needs clang + libbpf-devel"'
