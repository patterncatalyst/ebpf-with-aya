#!/usr/bin/env bash
#
# examples/16-container-targets/demo.sh
#
# 1. build contrace on the host, ship it to the target VM
# 2. copy the target apps to the VM and build/run them under Podman (on the VM)
# 3. resolve a container's cgroup id (best effort) and run contrace scoped to it
# 4. drive load from the host so the container opens files -> scoped events
#
#   ./demo.sh                 # full run against the FastAPI target
#   TARGET=quarkus ./demo.sh  # scope to the Quarkus target instead
#   ./demo.sh build           # just build contrace on the host

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; TARGET="${TARGET:-fastapi}"
BIN="$SCRIPT_DIR/target/release/contrace"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }

build(){ c_step "building contrace (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary at $BIN"; c_ok "built $BIN"; }

case "${1:-run}" in build) build; exit 0;; esac
build
IP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"
GW="$($SSH 'ip route | awk "/default/ {print \$3; exit}"')"
if [[ "$TARGET" == "quarkus" ]]; then CNAME=quarkus-target; PORT=8080; else CNAME=fastapi-target; PORT=8000; fi

c_step "shipping target apps to $VM and starting $CNAME under Podman (on the VM)"
$SSH 'mkdir -p ~/targets'
scp -r -o StrictHostKeyChecking=accept-new targets/"$TARGET" "fedora@$IP:~/targets/$TARGET"
$SSH "cd ~/targets/$TARGET && podman build -t ebpf-$TARGET-target:dev . && (podman rm -f $CNAME || true) && podman run -d --name $CNAME -p 127.0.0.1:$PORT:$PORT ebpf-$TARGET-target:dev && sleep 2 && podman ps --filter name=$CNAME"

c_step "resolving $CNAME cgroup id on the VM (best effort)"
CGID="$($SSH "cgp=\$(podman inspect --format '{{.State.CgroupPath}}' $CNAME 2>/dev/null); if [ -n \"\$cgp\" ] && [ -d \"/sys/fs/cgroup\$cgp\" ]; then stat -c %i \"/sys/fs/cgroup\$cgp\"; else echo 0; fi" || echo 0)"
if [[ "$CGID" == "0" || -z "$CGID" ]]; then
  c_info "couldn't auto-resolve the cgroup id (rootless layouts vary) — running UNSCOPED (all cgroups)."
  c_info "to scope manually: find it on the VM with"
  c_info "    podman inspect --format '{{.State.CgroupPath}}' $CNAME   then   stat -c %i /sys/fs/cgroup<path>"
  CGID=0
else
  c_ok "$CNAME cgroup id = $CGID"
fi

c_step "starting a load driver on the host (curling the target's /work)"
( for i in $(seq 1 600); do curl -s "http://$IP:$PORT/work?n=500" >/dev/null 2>&1 || true; sleep 0.3; done ) &
LOAD_PID=$!; trap 'kill $LOAD_PID 2>/dev/null || true' EXIT
c_info "load driver pid $LOAD_PID hitting http://$IP:$PORT/work"

c_step "running contrace on $VM, scoped to cgroup $CGID (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$CGID" "$CNAME"
