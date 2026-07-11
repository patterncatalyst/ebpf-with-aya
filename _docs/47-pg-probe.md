---
title: "Probing postgres: queries and lock waits"
order: 47
part: Application targets
description: "A harder target: a postgres database. Attach uprobes to a multi-process server so one probe covers every backend, capture per-query latency with the SQL text, and measure the lock-wait time that is invisible from outside the database. Meet USDT — the stable probe API databases ship — and cross-check against postgres's own statistics views."
duration: 45 minutes
---

A database is the hardest kind of target: a long-running, multi-process server
whose most important behavior — how long queries take, and how much time is
lost *waiting on locks* — is exactly the part you can't see from the outside.
A slow query log tells you a statement was slow; it doesn't tell you the
backend spent 80 ms blocked on a row lock another transaction held. eBPF can,
because it sees the database's internal functions execute. This chapter probes
**postgres**: per-query latency with the SQL text, and lock-wait time, keyed by
the backend process — and introduces **USDT**, the stable tracepoint API that
databases ship for exactly this.

The code is in `examples/47-pg-probe/`. `./demo.sh` there runs postgres, drives
queries and a lock-contention scenario, and attaches the probe; its
`README.md` has the details.

{% include excalidraw.html
   file="pg-probe"
   alt="Clients send SQL to postgres, which runs as one binary with many backend processes (backend pid A, backend pid B). A uprobe on exec_simple_query captures per-query duration and the SQL text; a uprobe on ProcSleep captures lock-wait time. Both are keyed by the backend pid and exported to Grafana as query latency with SQL text and time waiting on locks. One uprobe on the postgres binary covers every backend process, and the backend pid separates connections."
   caption="Figure 47.1 — One uprobe covers every backend; the pid separates connections" %}

## A multi-process target

postgres uses a process-per-connection model: a `postmaster` accepts
connections and forks a **backend** process for each, and that backend handles
the connection's queries one at a time, single-threaded. Two consequences make
it pleasant to probe:

- A uprobe attaches to an **inode**, and every backend is the same postgres
  binary, so **one uprobe covers all of them at once** — including backends
  that fork *after* you attach.
- Because a backend is single-threaded and runs one query at a time, the
  **backend pid is a perfect key**: no event-loop juggling like nginx, no
  socket bookkeeping. `bpf_get_current_pid_tgid()` at probe time tells you
  which connection's query you're timing.

## USDT: the stable probe API

We've been attaching to internal C functions by name, which works but is
fragile — function names and signatures change between releases. Databases
solve this by shipping **USDT** probes (User Statically Defined Tracing):
named, documented, stable trace points compiled into the binary at fixed
spots, recorded in an ELF `.note.stapsdt` section. postgres (built
`--enable-dtrace`) exposes probes like `query__start`, `query__done`,
`lock__wait__start`, and `lock__wait__done` — a contract that survives
refactors. You can list them:

```bash
[vm]$ readelf -n /usr/lib/postgresql/*/bin/postgres | grep -A2 stapsdt | head
[vm]$ sudo bpftrace -l 'usdt:/path/to/postgres:*'
```

USDT is the *right* long-term target for database observability. Aya's
first-class probes today are kprobes and uprobes, with USDT attachment still
maturing, so this chapter attaches uprobes to the functions those markers wrap
— `exec_simple_query` (which `query__start`/`query__done` bracket) and
`ProcSleep` (the lock wait) — and notes USDT as the stable alternative once the
tooling lands.

## How the code works

Four programs, paired into two timings, both keyed by the backend pid:

```rust
#[map] static QSTART: HashMap<u64, u64> = HashMap::with_max_entries(4096, 0);   // pid -> query start
#[map] static QTEXT:  HashMap<u64, [u8; 128]> = HashMap::with_max_entries(4096, 0); // pid -> SQL
#[map] static LSTART: HashMap<u64, u64> = HashMap::with_max_entries(4096, 0);   // pid -> lock-wait start
#[map] static EVENTS: RingBuf = RingBuf::with_byte_size(1 << 16, 0);

#[uprobe] // exec_simple_query(const char *query_string)
pub fn q_start(ctx: ProbeContext) -> u32 {
    let pid = bpf_get_current_pid_tgid();
    let qptr: u64 = ctx.arg(0).unwrap_or(0);
    let mut buf = [0u8; 128];
    if qptr != 0 {
        let _ = unsafe { bpf_probe_read_user_str_bytes(qptr as *const u8, &mut buf) };
        let _ = QTEXT.insert(&pid, &buf, 0);
    }
    let _ = QSTART.insert(&pid, &unsafe { bpf_ktime_get_ns() }, 0);
    0
}

#[uretprobe] // returns from exec_simple_query
pub fn q_done(_ctx: ProbeContext) -> u32 {
    let pid = bpf_get_current_pid_tgid();
    if let Some(&start) = unsafe { QSTART.get(&pid) } {
        emit(Kind::Query, pid, now() - start, unsafe { QTEXT.get(&pid).copied() });
        let _ = QSTART.remove(&pid);
        let _ = QTEXT.remove(&pid);
    }
    0
}
```

`ProcSleep` gets the same entry/return pair, emitting a `Kind::LockWait` event
with the time the backend was blocked. Reading the points:

- **`q_start`** reads the query string (`arg(0)`, a C pointer) into a fixed
  128-byte buffer with `bpf_probe_read_user_str_bytes`, stashes it keyed by
  pid, and stamps the start. We capture the text at entry because by the time
  the uretprobe fires, the argument register is gone.
- **`q_done`** looks up the start by pid, computes the duration, and emits the
  query event with its text. The user side turns these into a latency
  histogram and logs the SQL.
- **`ProcSleep`** entry→return is pure lock-wait time: a backend only calls it
  when it must block waiting for a lock another transaction holds. That
  duration is the contention you can't see from outside.

The 128-byte SQL capture is deliberately bounded — eBPF wants fixed sizes, and
truncating long statements is fine for a latency view. Treat captured SQL as
sensitive: it can contain literal values (a forward-ref to redaction, which
OBI and friends handle with route templating).

## Build, deploy, observe

```bash
cd examples/47-pg-probe && ./demo.sh
```

The demo runs a postgres container, drives a steady query workload, and then
stages a **lock-contention** scenario — two transactions fighting over the same
row — so the lock-wait probe has something to show. Resolve a backend pid,
attach the four uprobes to the in-container postgres binary, and watch:

- **In the terminal**, a live line per query (duration + truncated SQL) and per
  lock wait.
- **In Grafana** (`127.0.0.1:3000` → Explore), graph
  `histogram_quantile(0.95, sum by (le) (rate(ebpf_pg_query_duration_ms_bucket[1m])))`
  for p95 query latency, and `rate(ebpf_pg_lock_wait_ms_sum[1m])` to watch
  lock-wait time climb the moment the contention scenario runs — the signal a
  slow-query log alone would never give you.

## Cross-check

postgres keeps its own books, which is the perfect cross-check — compare what
eBPF measured against what the database reports:

```sql
-- queries by total/mean time (needs the pg_stat_statements extension)
SELECT query, calls, mean_exec_time FROM pg_stat_statements ORDER BY mean_exec_time DESC LIMIT 5;
-- live sessions currently blocked, and on what
SELECT pid, wait_event_type, wait_event, state FROM pg_stat_activity WHERE wait_event_type = 'Lock';
```

`pg_stat_statements.mean_exec_time` should track your `ebpf_pg_query_duration`
histogram, and a backend showing `wait_event_type = 'Lock'` in
`pg_stat_activity` should line up with a `ProcSleep` interval your probe timed.
When the database's own view agrees with the kernel's, the probe is faithful.

## What you learned

- A database is a **multi-process** target: one uprobe on the postgres binary
  covers every backend, and the **backend pid** is a clean per-connection key.
- eBPF sees what outside tools can't: per-query latency **with the SQL text**
  (`exec_simple_query`) and **lock-wait time** (`ProcSleep`) — contention made
  measurable.
- **USDT** is the stable, documented probe API databases ship
  (`query__start`, `lock__wait__start`, …); uprobes on the wrapped functions
  are the pragmatic stand-in until Aya's USDT attachment matures.

That closes the Application targets part. Next, Part 8 turns to the advanced
kernel surface — the BPF features that arrived in 2024–2026 — starting with
detaching and pinning programs so they outlive their loader.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
