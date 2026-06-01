---
title: "Correlating the signals: Tempo, Mimir, and one trace"
order: 62
part: Operating eBPF
description: "We have used the metrics half of the observability stack for sixty chapters and never touched the rest. The otel-lgtm backend also runs Tempo for traces and Loki for logs, and the real power is correlation: from a metric spike to the exact trace, from that span to its logs, from a request to the eBPF view of what the kernel did. Learn spans and trace context, what each backend is (and where Mimir fits), and how Grafana links them by trace_id — the groundwork for the capstone."
duration: 40 minutes
---

Every program in this book has fed Grafana the same kind of thing: an `ebpf_*`
**metric**. We graphed counters and gauges and never once looked at the other
two signals — and the backend has been quietly running them the whole time.
The `grafana/otel-lgtm` stack from Chapter 3 isn't just Prometheus; it bundles
**Tempo** for traces, **Loki** for logs, and **Pyroscope** for profiles, all
behind one OTel Collector and one Grafana. The reason that matters isn't "more
dashboards" — it's **correlation**: the ability to start at a metric spike,
jump to the exact distributed **trace** that caused it, read that request's
**logs**, and line it all up against the **eBPF** view of what the kernel did
for that same request. This chapter builds that picture; the next one (the
capstone) puts a real request through all of it.

The code is in `examples/62-correlation/`. `./demo.sh` runs a traced FastAPI
endpoint, hits it with `curl`, and walks the metric → trace → logs path in
Grafana; the `README.md` has the details.

{% include excalidraw.html
   file="signal-correlation"
   alt="Every signal carries the trace_id — Grafana turns four stores into one investigation. Sources on the left — Quarkus in Podman, FastAPI in Podman, and eBPF/Aya — all send OTLP to the OTel Collector on ports 4317 and 4318. The collector fans out to Prometheus for metrics (Mimir in production), Tempo for traces and spans, and Loki for logs plus Pyroscope for profiles. At the bottom, Grafana links every signal by trace_id: exemplars take you from a metric to a trace, tracesToLogs from a span to logs, derivedFields from a log back to a trace, and traces to profiles. The lab's otel-lgtm uses Prometheus; Mimir is the Prometheus-compatible production swap with the same PromQL and the same correlation."
   caption="Figure 62.1 — Four backends, one trace_id: the correlation web Grafana stitches together" %}

## The three signals and four stores

OpenTelemetry standardized three **signals** — metrics, traces, logs — plus a
fourth, profiles, arriving now. The stack stores each in a purpose-built
backend, all fed by the **OTel Collector** over OTLP (gRPC 4317 / HTTP 4318):

- **Metrics → Prometheus.** Time series — our `ebpf_*` counters and gauges.
  This is what we have used all along, queried with PromQL.
- **Traces → Tempo.** Distributed traces: a **trace** is one request's journey,
  made of **spans** (each a timed operation — an HTTP handler, a DB call), linked
  parent-to-child and tagged with a shared **trace_id**. Tempo ingests OTLP (and
  Jaeger/Zipkin), stores cheaply on object storage, and can even *generate*
  metrics and service graphs from the spans.
- **Logs → Loki.** Log lines, indexed by labels rather than full text — and,
  crucially, carrying the **trace_id** of the request that emitted them.
- **Profiles → Pyroscope.** Continuous CPU/memory profiles, the newest signal.

The unifying thread across all four is the **trace_id**: a metric exemplar
points at one, a log line carries one, a span *is* identified by one. Correlation
is just following that id between stores.

## Where Mimir fits

You asked the right question: the `M` in "LGTM" stands for **Mimir**, yet the
lab runs **Prometheus**. Both are true. The `otel-lgtm` image is a
development/demo backend optimized for a five-second startup, so it uses
single-binary **Prometheus** for metrics. **Mimir** is what you graduate to in
production: a horizontally scalable, multi-tenant, long-retention metrics store
that ingests via **Prometheus remote-write** and serves the **same PromQL**. The
swap is deliberately invisible to everything above it — Grafana points at Mimir
with the *Prometheus* data-source type (`url: .../prometheus`), your `ebpf_*`
queries are unchanged, and exemplar-based correlation works identically. So:
learn and demo on Prometheus, deploy on Mimir when you need 1B-series scale and
multi-tenancy; nothing in this book's queries changes.

## Trace context: how a request stays one trace

A trace stays coherent across services because of **trace context propagation**.
The W3C standard defines a `traceparent` HTTP header carrying the trace_id and
the current span id; when a service receives a request it continues that trace,
and when it calls another service it forwards the header. So a `curl` that
injects a `traceparent` (or an instrumented client that starts one) produces a
single trace whose spans span every hop:

```bash
curl -H "traceparent: 00-$(openssl rand -hex 16)-$(openssl rand -hex 8)-01" \
     http://127.0.0.1:8000/work
```

The receiving FastAPI or Quarkus app, with its OTel SDK active, picks that up,
creates spans under it, emits metrics, and writes logs stamped with the same
trace_id. One id, every signal.

## How Grafana links them

Correlation isn't magic in the data — it's **data-source linking** configured in
Grafana, and it's worth seeing the actual wiring because it demystifies the
"click from metric to trace" experience:

- **Exemplars (metric → trace).** Prometheus/Mimir series can carry *exemplars*
  — sample points annotated with a trace_id. Grafana's Prometheus data source
  declares `exemplarTraceIdDestinations: [{ name: trace_id, datasourceUid: tempo }]`,
  so an exemplar dot on a latency graph is a one-click jump to that trace.
- **Traces → logs / metrics.** Tempo's data source declares `tracesToLogs`
  (open Loki filtered to this trace's id and service) and `tracesToMetrics`, so
  from a slow span you reach its logs and related metrics.
- **Logs → traces.** Loki's data source declares `derivedFields` — a regex that
  finds `trace_id=…` in a log line and turns it into a link into Tempo.
- **Traces → profiles.** Tempo links to Pyroscope for the CPU profile of a span.

Wire those once (the example ships the data-source file) and four separate
stores become a single investigation surface.

## Where eBPF and Aya join the trace

Here's the part unique to this book, and the hinge for the capstone. An
application's OTel SDK knows the trace_id; **the kernel does not.** An eBPF
program watching a syscall or a socket sees bytes and pids, not your request's
trace_id — so how does the eBPF view join the trace? Two ways, both seen earlier:

- **Capture the id from the wire or the handler.** An L7 parse (Chapter 29) or a
  uprobe on the request handler (Chapters 14, 45) can read the `traceparent`
  header or the in-process trace_id and attach it to the eBPF event — so an
  `ebpf_*` metric or event carries the same trace_id the app reported, and lines
  up by exemplar.
- **Let OBI do it.** OpenTelemetry eBPF Instrumentation (Chapter 46) propagates
  trace context at the eBPF level zero-code, producing spans for traffic it sees
  without touching the app.

Failing an explicit id, you can still correlate **by time, pid, and service
attributes** — the eBPF metric and the app span share a wall-clock window and a
service name. The capstone uses both: the app supplies the trace_id, and the
eBPF layer is tagged and time-aligned so one request reads top-to-bottom, from
the HTTP span down to the kernel.

## Build, deploy, observe

```bash
cd examples/62-correlation && ./demo.sh
```

The demo runs a small **instrumented FastAPI** service in a Podman container
(OTel SDK → Collector), hits `/work` with `curl` carrying a `traceparent`, and
the service emits a span, a metric, and a log line — all stamped with that one
trace_id. It then prints the **Grafana navigation**: Explore → Tempo, find the
trace; click **Logs** on the span to land in Loki; from the request-rate metric,
click the **exemplar** to return to the trace. The data-source correlation file
the demo installs is what makes those clicks work.

## Cross-check

```bash
[host]$ curl -s http://127.0.0.1:3200/api/echo            # Tempo is up (query API on 3200)
[host]$ curl -s 'http://127.0.0.1:3200/api/search?limit=5' | jq '.traces[].traceID'  # recent traces
# In Grafana (127.0.0.1:3000): Explore → Tempo → paste a traceID → see spans → click Logs
```

Finding your `curl`'s trace in Tempo by its id, and reaching its log line in
Loki from the span, is the proof the correlation is wired — the same id you
generated on the command line, followed all the way through the stack.

## What you learned

- The `otel-lgtm` stack stores the three signals in **Prometheus** (metrics),
  **Tempo** (traces/spans), and **Loki** (logs), plus **Pyroscope** (profiles);
  **Mimir** is the Prometheus-compatible production swap for metrics — same
  PromQL, same correlation, bigger scale.
- A **trace** is one request as parent/child **spans** sharing a **trace_id**,
  kept coherent across services by the W3C **`traceparent`** header; Grafana
  links the stores with **exemplars** (metric→trace), **tracesToLogs**, and
  **derivedFields** (log→trace).
- The **kernel doesn't know the trace_id**, so eBPF joins the trace by
  **capturing it** (L7/uprobe) or via **OBI**, or correlates by **time + pid +
  service** — the bridge the capstone builds next.

Next, Chapter 63 is the **capstone**: one `curl` through Quarkus and FastAPI,
observed at every layer — app spans, metrics, logs, and the eBPF/Aya kernel
view — correlated on a single trace.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that the FastAPI container exports OTLP to the
Chapter 3 stack; that the trace appears in Tempo (query API on 3200) and the
span links to its Loki logs; that the Grafana data-source correlation file
provisions exemplar/tracesToLogs/derivedFields links; and that a `traceparent`
on `curl` yields one coherent trace.*
