# 47 · Probing postgres: queries and lock waits

Attach uprobes to a multi-process **postgres** server to measure per-query
latency (with the SQL text) and **lock-wait time** — the contention you can't
see from outside the database.

## Pieces

- `pg-probe-ebpf` — uprobe/uretprobe pairs on `exec_simple_query` (query
  latency + SQL via `bpf_probe_read_user_str_bytes`) and `ProcSleep` (lock
  wait), keyed by the backend pid; events to a ring buffer.
- `pg-probe-common` — the shared `Event`.
- `pg-probe` — drains the ring; exports `ebpf_pg_query_duration_ms` and
  `ebpf_pg_lock_wait_ms`; prints queries + lock waits live.

## Run it

```bash
./demo.sh          # run postgres on $VM, drive queries + lock contention, attach
./demo.sh build    # just build the probe on the host
```

One uprobe on the postgres binary covers **every backend** (attach with
`pid=None`); the backend pid separates connections.

## Symbols (important)

uprobes need `exec_simple_query` / `ProcSleep` in the binary's symbol table.
Stock postgres images are usually stripped — install the matching debug
symbols, or use a `--enable-dtrace` build (which also gives you the stable
`query__start` / `lock__wait__start` **USDT** probes). Check:

```bash
nm /proc/$(pgrep -x postgres|head -1)/root$(readlink /proc/$(pgrep -x postgres|head -1)/exe) | grep exec_simple_query
readelf -n <postgres-binary> | grep -A2 stapsdt    # USDT probes, if built with dtrace
```

## Cross-check (postgres's own books)

```sql
SELECT query, calls, mean_exec_time FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 5;
SELECT pid, wait_event_type, wait_event, state FROM pg_stat_activity WHERE wait_event_type = 'Lock';
```

## Verification status

**Unverified.** Confirm `exec_simple_query` / `ProcSleep` symbol presence, the
`bpf_probe_read_user_str_bytes` helper and `arg(0)` query string, the
`ProcSleep` signature across versions, and that the latency/lock-wait series
track `pg_stat_statements` / `pg_stat_activity`.
