#!/usr/bin/env bash
# examples/20-javagc/demo.sh
# Build javagc, ship a small allocating Java program to the VM, run it under a
# JDK (small heap -> frequent GC), resolve the HotSpot USDT gc__begin/gc__end
# probe offsets in libjvm.so, and time GC pauses.
#
# Requires a JDK on the VM (Fedora: sudo dnf install -y java-latest-openjdk-devel).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
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
$SSH 'command -v javac >/dev/null || { echo "install a JDK: sudo dnf install -y java-latest-openjdk-devel"; exit 1; }'
scp -o StrictHostKeyChecking=accept-new target-java/Alloc.java "fedora@$IP:/home/fedora/Alloc.java"
$SSH 'cd /home/fedora && javac Alloc.java && pkill -f "java .*Alloc" || true; nohup java -Xmx64m -XX:+UseG1GC -XX:+ExtendedDTraceProbes Alloc >/tmp/alloc.log 2>&1 & echo started pid $!'

c_step "resolving libjvm.so + USDT gc probe offsets on the VM"
LIBJVM="$($SSH 'f=$(find /usr/lib/jvm -name libjvm.so 2>/dev/null | head -1); echo "$f"')"
[[ -n "$LIBJVM" ]] || c_fail "could not find libjvm.so on the VM (install a JDK)"
c_ok "libjvm: $LIBJVM"
# stapsdt notes give each probe's Location (vaddr). For a shared object the
# uprobe file offset typically equals that vaddr; verify if attach fails.
read BEG END < <($SSH "readelf -n '$LIBJVM' 2>/dev/null | awk '
  /Provider: hotspot/ {prov=1}
  /Name: gc__begin/ {getloc=\"begin\"}
  /Name: gc__end/ {getloc=\"end\"}
  /Location:/ { if(getloc==\"begin\"&&!b){b=strtonum(\$2)} else if(getloc==\"end\"&&!e){e=strtonum(\$2)}; getloc=\"\" }
  END{print b\" \"e}'")
if [[ -z "${BEG:-}" || -z "${END:-}" || "$BEG" == "0" ]]; then
  c_info "couldn't auto-resolve USDT offsets. Confirm the probes exist + see them working with bpftrace (definitely supports USDT):"
  c_info "    ssh fedora@$IP \"sudo bpftrace -e 'usdt:$LIBJVM:hotspot:gc__begin { @=count(); }'\""
  c_fail "re-run after resolving BEGIN/END offsets, or pass them: deploy + run 'javagc $LIBJVM <begin> <end>'"
fi
c_ok "gc__begin@$BEG  gc__end@$END"

c_step "attaching javagc and timing GC pauses (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$LIBJVM" "$BEG" "$END"
