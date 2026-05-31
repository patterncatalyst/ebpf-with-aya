#!/usr/bin/env bash
# examples/51-userspace-ebpf/demo.sh — runs ON THE HOST. No lab VM, no root,
# no kernel: eBPF bytecode executed in a user-space VM (rbpf). That absence of
# ceremony is the whole point of this chapter.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
c_step "building (host)"; cargo build --release || c_fail "cargo build failed"
case "${1:-run}" in build) exit 0;; esac
c_step "disassembly — it's ordinary eBPF"
cargo run --release --quiet -- --disasm || true
c_step "run interpreter + JIT (no kernel, no root)"
cargo run --release --quiet
