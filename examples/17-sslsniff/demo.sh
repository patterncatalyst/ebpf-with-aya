#!/usr/bin/env bash
# examples/17-sslsniff/demo.sh — build sslsniff, deploy to VM, run it, then
# drive TLS traffic on the VM (a local openssl server + curl) so plaintext
# crosses SSL_write/SSL_read.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/sslsniff"
LIBSSL="${LIBSSL:-/usr/lib64/libssl.so.3}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building sslsniff (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"
GW="$($SSH 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "verify the symbol exists in libssl on the VM:"
c_info "    ssh fedora@$IP 'nm -D $LIBSSL | grep -E \" SSL_(read|write)\$\"'"
c_info "drive TLS on the VM in another terminal, e.g.:"
c_info "    ssh fedora@$IP 'curl -sk https://127.0.0.1:8443/ >/dev/null'  (needs a local TLS server)"
c_info "or simply: ssh fedora@$IP 'curl -s https://localhost.localdomain 2>/dev/null; openssl s_client -connect 127.0.0.1:443 </dev/null 2>/dev/null'"
c_step "deploying sslsniff to $VM, attaching to $LIBSSL (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$LIBSSL"
