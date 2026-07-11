#!/usr/bin/env bash
# examples/47-pg-probe/demo.sh — run postgres on the TARGET, drive queries and
# a lock-contention scenario, and attach uprobes to observe per-query latency
# (with SQL) and lock-wait time. NOTE: needs a postgres binary with symbols for
# exec_simple_query / ProcSleep (install debug symbols or a --enable-dtrace
# build); the stock image is often stripped — see README.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
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
$SSH "fedora@$TIP" "podman rm -f ebpf-pg 2>/dev/null; podman run -d --name ebpf-pg -e POSTGRES_PASSWORD=demo -p 5432:5432 $PG_IMAGE >/dev/null && echo postgres starting"
$SSH "fedora@$TIP" "for i in \$(seq 1 30); do podman exec ebpf-pg pg_isready -U postgres >/dev/null 2>&1 && break; sleep 1; done; $PSQL 'CREATE TABLE IF NOT EXISTS t(id int primary key, v int); INSERT INTO t VALUES (1,0) ON CONFLICT DO NOTHING;' >/dev/null && echo seeded"
# steady query load
$SSH "fedora@$TIP" "nohup bash -c 'while true; do $PSQL \"SELECT count(*) FROM t; SELECT pg_sleep(0.01);\" >/dev/null 2>&1; sleep 0.1; done' </dev/null >/dev/null 2>&1 & echo driving query load"
# lock contention: one tx holds the row, another waits on it (fires ProcSleep)
$SSH "fedora@$TIP" "nohup bash -c 'while true; do podman exec -e PGPASSWORD=demo ebpf-pg psql -U postgres -c \"BEGIN; UPDATE t SET v=v+1 WHERE id=1; SELECT pg_sleep(2); COMMIT;\" </dev/null >/dev/null 2>&1 & sleep 0.3; podman exec -e PGPASSWORD=demo ebpf-pg psql -U postgres -c \"UPDATE t SET v=v+1 WHERE id=1;\" >/dev/null 2>&1; wait; sleep 1; done' </dev/null >/dev/null 2>&1 & echo staging lock contention"
sleep 2
WPID="$($SSH "fedora@$TIP" "pgrep -f 'postgres:.*' | head -1 || pgrep -x postgres | head -1")"
[ -n "$WPID" ] || c_fail "no postgres backend pid found on $VM"
INPATH="$($SSH "fedora@$TIP" "readlink /proc/$WPID/exe")"
TARGET="/proc/$WPID/root$INPATH"
c_info "backend pid=$WPID  postgres binary=$TARGET"
$SSH "fedora@$TIP" "nm $TARGET 2>/dev/null | grep -q exec_simple_query && echo 'symbols present ✓' || echo 'WARNING: symbols not found — install postgres debug symbols or use a --enable-dtrace build'"
c_step "deploying pg-probe to $VM (Ctrl-C to stop)"
OTEL_ENDPOINT="http://$GW:4318" "$LAB/deploy-to-target.sh" "$VM" "$BIN" -- "$TARGET"
