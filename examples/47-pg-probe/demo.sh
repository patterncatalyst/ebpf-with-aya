#!/usr/bin/env bash
# examples/47-pg-probe/demo.sh — run postgres on the TARGET, drive queries and
# a lock-contention scenario, and attach uprobes to observe per-query latency
# (with SQL) and lock-wait time. The stock postgres image is stripped, so the
# demo builds a symboled image (Containerfile adds the dbgsym package) and wires
# the dbgsym debug info merged into a bind-mounted host copy of the binary (eu-unstrip),
# so aya can resolve exec_simple_query / ProcSleep. See README.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
source "$REPO_ROOT/scripts/lib/_demo-bg.sh"   # reap guest-side load-gens on exit
VM="${VM:-ebpf-target}"; BIN="$SCRIPT_DIR/target/release/pg-probe"
PG_IMAGE="${PG_IMAGE:-docker.io/library/postgres:17}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }; c_fail(){ echo -e "\033[0;31m✗ $*\033[0m" >&2; exit 1; }
build(){ c_step "building pg-probe (release)"; cargo build --release || c_fail "cargo build failed"; [[ -x "$BIN" ]] || c_fail "no binary"; c_ok "built $BIN"; }
case "${1:-run}" in build) build; exit 0;; esac
build
TIP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new"
GW="$($SSH "fedora@$TIP" 'ip route | awk "/default/{print \$3; exit}"')"
PSQL='podman exec -e PGPASSWORD=demo ebpf-pg psql -U postgres -tAc'
c_info "target=$TIP OTLP=http://$GW:4318  (postgres + query load + lock contention)"
# Build a symboled postgres (dbgsym), extract the server binary + its split-debug
# file, and merge them with `eu-unstrip` into one fully-symboled binary — so the
# symbols land in the binary's own .symtab with real .text offsets (aya's uprobe
# resolver needs that; a bare .gnu_debuglink .debug has a NOBITS .text and fails).
# Rootless podman hides a container's binary from the host namespace, so we
# bind-mount this host copy in and point the uprobe at it.
scp -q -o StrictHostKeyChecking=accept-new "$SCRIPT_DIR/Containerfile" "fedora@$TIP:/tmp/pg.Containerfile"
$SSH "fedora@$TIP" 'cd /tmp && podman build -q -t ebpf-pg-sym -f pg.Containerfile . >/dev/null && echo "symboled postgres image built"'
$SSH "fedora@$TIP" 'set -e; command -v eu-unstrip >/dev/null || sudo dnf install -y elfutils >/dev/null 2>&1; cid=$(podman create ebpf-pg-sym); podman cp "$cid:/usr/lib/postgresql/17/bin/postgres" /tmp/ebpf-pg-bin; rm -rf /tmp/pgdebug; mkdir -p /tmp/pgdebug; podman cp "$cid:/usr/lib/debug" /tmp/pgdebug; podman rm "$cid" >/dev/null; BID=$(readelf -n /tmp/ebpf-pg-bin | awk "/Build ID/{print \$3}"); eu-unstrip /tmp/ebpf-pg-bin "/tmp/pgdebug/debug/.build-id/${BID:0:2}/${BID:2}.debug" -o /tmp/ebpf-pg-merged; echo "staged symboled postgres binary"'
$SSH "fedora@$TIP" "podman rm -f ebpf-pg 2>/dev/null; podman run -d --name ebpf-pg -e POSTGRES_PASSWORD=demo -v /tmp/ebpf-pg-merged:/usr/lib/postgresql/17/bin/postgres:ro,Z -p 5432:5432 ebpf-pg-sym >/dev/null && echo postgres starting"
$SSH "fedora@$TIP" "for i in \$(seq 1 30); do podman exec ebpf-pg pg_isready -U postgres >/dev/null 2>&1 && break; sleep 1; done; $PSQL 'CREATE TABLE IF NOT EXISTS t(id int primary key, v int); INSERT INTO t VALUES (1,0) ON CONFLICT DO NOTHING;' >/dev/null && echo seeded"
# steady query load
$SSH "fedora@$TIP" "nohup bash -c 'while true; do $PSQL \"SELECT count(*) FROM t; SELECT pg_sleep(0.01);\" >/dev/null 2>&1; sleep 0.1; done' </dev/null >/dev/null 2>&1 & echo driving query load"
reap "fedora@$TIP" 'SELECT count(*) FROM t; SELECT pg_sleep(0.01)'
# lock contention: one tx holds the row, another waits on it (fires ProcSleep)
$SSH "fedora@$TIP" "nohup bash -c 'while true; do podman exec -e PGPASSWORD=demo ebpf-pg psql -U postgres -c \"BEGIN; UPDATE t SET v=v+1 WHERE id=1; SELECT pg_sleep(2); COMMIT;\" </dev/null >/dev/null 2>&1 & sleep 0.3; podman exec -e PGPASSWORD=demo ebpf-pg psql -U postgres -c \"UPDATE t SET v=v+1 WHERE id=1;\" >/dev/null 2>&1; wait; sleep 1; done' </dev/null >/dev/null 2>&1 & echo staging lock contention"
reap "fedora@$TIP" 'BEGIN; UPDATE t SET v=v+1 WHERE id=1; SELECT pg_sleep(2)'
sleep 2
# The container executes the bind-mounted, fully-symboled host copy; aya reads
# exec_simple_query / ProcSleep straight from its .symtab. pid=None in the loader
# covers every backend, so we don't need a specific pid.
TARGET="/tmp/ebpf-pg-merged"
c_info "postgres binary=$TARGET  (fully symboled: dbgsym merged in with eu-unstrip)"
$SSH "fedora@$TIP" "nm $TARGET 2>/dev/null | grep -q exec_simple_query && echo 'symbols present ✓' || echo 'WARNING: exec_simple_query not in symtab — symbol resolution will fail'"
c_step "deploying pg-probe to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$TARGET"
