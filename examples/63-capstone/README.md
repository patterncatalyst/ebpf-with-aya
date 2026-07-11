# 63 · Capstone: one request, every layer

One `curl` with a `traceparent` → **FastAPI /checkout** → **Quarkus /inventory**,
producing one distributed trace (Tempo), metrics (Prometheus), and logs (Loki),
while an **Aya** observer adds the kernel-side fourth view — all on one trace_id.

## Pieces

- `fastapi-app/` — `/checkout` (Podman/UBI py3.14): span + metric + log, calls Quarkus.
- `quarkus-app/` — `/inventory` (Podman/UBI Java 25 + Quarkus 3.33), auto-instrumented.
- `capstone-ebpf` / `capstone` — Aya observer: per-command socket-read counts →
  `ebpf_capstone_syscalls_total{comm}`.
- `reference/l7_traceparent.bpf.c` — canonical L7 trace_id extraction (OBI-style).
- `compose.yaml` — both services; OTLP → the Chapter 3 stack.

## Run it

```bash
./demo.sh          # bring up apps + observer, fire one traced curl, print the trace_id + Grafana path
```

## Read one request end to end (Grafana, 127.0.0.1:3000)

1. Explore → **Tempo** → paste the trace_id → spans from **both** services.
2. Span → **Logs** (Loki); metric **exemplar** → trace.
3. Graph `ebpf_capstone_syscalls_total` for the window — the kernel's view.

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built and run on the lab VM host: both
services build and come up under podman-compose, and one traceparented request
flows FastAPI `/checkout` → Quarkus `/inventory` across both services, while the
Aya observer attaches and emits `ebpf_capstone_*`. The L7 `traceparent` reference
(`reference/l7_traceparent.bpf.c`) remains canonical-but-unexercised; the runnable
observer correlates by command/time. Container image tags (UBI openjdk-25, Quarkus
3.33) and struct offsets can be version-specific.
