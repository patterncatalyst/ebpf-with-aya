#!/usr/bin/env bash
# examples/20-javagc/demo.sh
# Build javagc, ship a small allocating Java program to the VM, run it under a
# JDK (small heap -> frequent GC), resolve the G1 stop-the-world pause symbol in
# libjvm.so, and time GC pauses with a uprobe+uretprobe.
#
# Fedora's OpenJDK isn't built with --enable-dtrace, so the hotspot USDT gc
# markers don't exist — we uprobe the collector's real pause function instead,
# resolved from libjvm.so's (unstripped) .symtab.
# Requires a JDK on the VM (Fedora: sudo dnf install -y java-latest-openjdk-devel).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
source "$REPO_ROOT/scripts/lib/_demo-bg.sh"   # reap guest-side load-gens on exit
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/javagc"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building javagc (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"
GW="$($SSH 'ip route | awk "/default/ {print \$3; exit}"')"
c_info "OTLP -> http://$GW:4318"

c_step "shipping + compiling Alloc.java on the VM, starting it with a small heap"
$SSH 'command -v javac >/dev/null || { echo "installing a JDK (one-time)…"; sudo dnf install -y java-latest-openjdk-devel >/dev/null 2>&1; }'
scp -o StrictHostKeyChecking=accept-new target-java/Alloc.java "fedora@$IP:/home/fedora/Alloc.java"
$SSH 'cd /home/fedora && javac Alloc.java && pkill -x java || true; nohup java -Xmx64m -XX:+UseG1GC Alloc </dev/null >/tmp/alloc.log 2>&1 & echo started pid $!'
reap "fedora@$IP" java

c_step "resolving libjvm.so + the G1 pause symbol on the VM"
LIBJVM="$($SSH 'f=$(find /usr/lib/jvm -name libjvm.so 2>/dev/null | head -1); echo "$f"')"
[[ -n "$LIBJVM" ]] || c_fail "could not find libjvm.so on the VM (install a JDK)"
c_ok "libjvm: $LIBJVM"
# The G1 stop-the-world pause entry point lives in libjvm.so's .symtab as a
# local C++ symbol. Its mangled name is JDK-version-specific, so resolve it
# dynamically rather than hard-coding. (nm reads .symtab; the release libjvm
# keeps its symbol table — only .debug is split out via .gnu_debuglink.)
SYM="$($SSH "nm '$LIBJVM' 2>/dev/null | awk '/G1CollectedHeap.*do_collection_pause_at_safepoint/{print \$3; exit}'")"
if [[ -z "${SYM:-}" ]]; then
  c_info "no do_collection_pause_at_safepoint symbol in this libjvm.so .symtab."
  c_info "This chapter needs a JDK whose libjvm keeps its symbol table (stock"
  c_info "Fedora OpenJDK does). Confirm:  ssh fedora@$IP \"nm $LIBJVM | grep do_collection_pause\""
  c_fail "could not resolve the G1 pause symbol."
fi
c_ok "G1 pause symbol: $SYM"

c_step "attaching javagc (uprobe+uretprobe) and timing GC pauses (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$LIBJVM" "$SYM"
