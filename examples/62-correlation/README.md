# 62 · Correlating the signals: Tempo, Mimir, and one trace

The otel-lgtm stack from Chapter 3 stores three signals — metrics
(**Prometheus**; **Mimir** in prod), traces (**Tempo**), logs (**Loki**) — plus
profiles (**Pyroscope**). The power is **correlation**: follow one **trace_id**
across all of them.

## Pieces

- `app/` — an instrumented **FastAPI** service (Podman/UBI, Python 3.14) that
  emits a span (Tempo), `app_requests_total` (Prometheus), and a trace-stamped
  log (Loki) on each `/work` request.
- `grafana/provisioning/datasources/correlation.yaml` — the data-source wiring
  that links them: exemplars (metric→trace), tracesToLogs (span→logs),
  derivedFields (log→trace).

## Run it

```bash
./demo.sh          # build + run the app, curl /work with a traceparent, print the Grafana path
./demo.sh build
```

Then in Grafana (`127.0.0.1:3000`): Explore → Tempo → paste the trace id → open
the span → **Logs** → Loki; and graph `app_requests_total`, click an **exemplar**
to jump back to the trace.

## Where eBPF joins (the capstone, next)

The kernel doesn't know the app's trace_id; eBPF joins by **capturing it** (L7
parse / uprobe — Chapters 14, 29, 45), via **OBI** (Chapter 46), or by
correlating on **time + pid + service**. Chapter 63 does the full thing.

## Verification status

**Unverified.** Confirm the FastAPI container exports OTLP to the Chapter 3
stack; that the trace appears in Tempo (3200) and links to Loki logs; that the
correlation data-source file provisions the exemplar/tracesToLogs/derivedFields
links; and that a `traceparent` on `curl` yields one coherent trace.
