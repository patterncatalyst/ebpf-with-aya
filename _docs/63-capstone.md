---
title: "Capstone: one request, every layer"
order: 63
part: Operating eBPF
description: "Everything the book has built, in a single request. A curl carrying a traceparent hits a FastAPI service that calls a Quarkus service; both emit spans, metrics, and logs on one trace_id, while an Aya program watches the same request at the socket layer and ties its eBPF view into that trace. The result is one investigation that reads from the HTTP span all the way down to the kernel — the payoff of sixty-two chapters."
duration: 60 minutes
---

This is the chapter the whole book has been walking toward. Individually you can
now write a kprobe, a uprobe, an XDP filter, an LSM hook; emit metrics, drain a
ring buffer, pin a program, relocate it with CO-RE; and stand up Tempo, Loki,
and Prometheus behind Grafana. The capstone puts them in one place: a **single
request**, triggered by `curl` or Postman, traced from the HTTP handler in one
service, through a call to a second service, and **down into the kernel** via an
Aya program — all stitched together on **one trace_id**. When you can click a
slow span in Tempo and see, for that exact request, the app's logs, its metrics,
*and* what the kernel did underneath it, you've built the thing eBPF and
OpenTelemetry exist to make possible.

The code is in `examples/63-capstone/`. `./demo.sh` brings up both services and
the eBPF observer, fires one traced `curl`, and shows the trace_id across every
store; the `README.md` has the details.

{% include excalidraw.html
   file="capstone"
   alt="The capstone: one request, every layer, one trace_id. A curl or Postman call with a traceparent hits FastAPI /checkout in Podman, which calls Quarkus /inventory in Podman, propagating the traceparent. Both apps send OTLP to the OTel Collector. An eBPF/Aya socket and L7 observer watches the sockets, extracts the traceparent, and emits ebpf_* metrics per request with the trace_id. The collector fans out to Tempo (spans from both services), Prometheus (app plus ebpf metrics), and Loki (trace-stamped logs); the eBPF metrics with their trace_id also go to Prometheus. At the bottom Grafana correlates everything on one trace_id: app spans, the eBPF view, metrics, and logs. App SDKs supply the trace_id; the eBPF layer extracts it from L7 or correlates by time, pid, and service — kernel to span in one view."
   caption="Figure 63.1 — One request, observed from the HTTP span down to the kernel, joined on a single trace_id" %}

## The scenario

A deliberately small but real shape: a `/checkout` request. The **FastAPI**
service receives it, and to fulfil it calls a **Quarkus** `/inventory` service —
two hops, two languages, two containers, one trace. Both run under Podman
(Chapters 45–47's pattern), both export OTLP to the Chapter 3 stack. We fire it
with one command:

```bash
curl -H "traceparent: 00-$(openssl rand -hex 16)-$(openssl rand -hex 8)-01" \
     http://127.0.0.1:8000/checkout
```

Because the `traceparent` propagates from FastAPI to Quarkus (their OTel SDKs do
this automatically), the result in Tempo is a **single trace** with spans from
*both* services nested under the incoming request — the distributed trace
Chapter 62 described, now real.

## Three signals from the apps

Each service, instrumented exactly as Chapter 62's example was, contributes all
three application signals under that one trace_id:

- **Spans → Tempo.** `checkout` (FastAPI) as the parent, an HTTP client span for
  the call to Quarkus, and `inventory` (Quarkus) as the child — the request's
  shape and timing, hop by hop.
- **Metrics → Prometheus.** `app_requests_total` and latency histograms from
  each service, with **exemplars** carrying the trace_id so a latency spike links
  straight to the trace.
- **Logs → Loki.** Each service's log line for the request, stamped with the
  trace_id (the `otelTraceID` field) so Grafana's derived field links it back to
  the span.

That alone is a complete application-observability story. The book's
contribution is the fourth view.

## The fourth view: eBPF underneath

While the apps describe themselves from the inside, an **Aya program watches the
same request from underneath** — at the socket layer, where the bytes actually
move. The observer counts the request's read/write syscalls and bytes for the
service processes and emits `ebpf_capstone_*` metrics, giving the kernel-side
truth (how many syscalls, how much data, how long on-CPU) that no application
SDK can see.

The crucial step is joining that view to the trace, and there are two honest
ways, both built earlier in the book:

- **Extract the trace_id at L7.** The `traceparent` travels in the HTTP request
  bytes. An eBPF program reading the socket payload — the OBI technique from
  Chapter 46, or a uprobe on the TLS/read path from Chapters 14–15 — can scan for
  `traceparent: 00-<trace_id>-…` and tag its event with that exact id. The
  example ships this as the canonical `reference/l7_traceparent.bpf.c`; it's the
  same id the app reported, captured from the wire.
- **Correlate by time, pid, and service.** Where extracting the id is
  impractical (encrypted payloads without a uprobe, an interpreter that resists
  probing), the eBPF metrics still share a wall-clock window, the container's
  pid/cgroup, and the `service.name` with the span — enough for Grafana to line
  them up. The runnable observer uses this path; the demo captures the trace_id
  from the response and tags the window so the correlation is exact.

Either way, the outcome is the headline: an `ebpf_capstone_*` series you can put
on the same Grafana view as the trace, for the *same* request.

## Reading one request end to end

With the stack wired (Chapter 62's correlation data sources) and the request
fired, the investigation reads top to bottom:

1. **Tempo** — find the trace by its id: the `checkout` span, the client call,
   the nested `inventory` span, with timings.
2. **Span → Logs** — click through to Loki for each service's log line for this
   request.
3. **Metric exemplar → trace** — from `app_requests_total`'s graph, the exemplar
   dot returns you to this trace.
4. **The eBPF layer** — alongside, `ebpf_capstone_syscalls_total` and on-CPU
   time for the request window show what the kernel did to serve it — the read
   syscalls on each socket, the bytes moved — the layer the app can't report.

That is the full picture: application intent (spans), application state (metrics,
logs), and kernel reality (eBPF), for one request, in one place.

## Build, deploy, observe

```bash
cd examples/63-capstone && ./demo.sh
```

The demo builds and runs both services with `podman-compose`, starts the Aya
observer, fires the traced `curl`, prints the resulting **trace_id**, and lists
the Grafana steps to read it across all four views. **In Grafana**
(`127.0.0.1:3000`): Explore → Tempo → the trace; then the linked logs, the
metric exemplars, and `ebpf_capstone_*` for the same window. It's the
three-signals capstone of Chapter 46, completed — two real services, the full
LGTM correlation, and the eBPF view joined in.

## Cross-check

```bash
[host]$ curl -s "http://127.0.0.1:3200/api/traces/<trace_id>" | jq '.batches | length'  # spans from both services
[host]$ curl -s 'http://127.0.0.1:9090/api/v1/query?query=ebpf_capstone_syscalls_total' | jq '.data.result'
[host]$ podman ps                                   # both app containers + their pids for the eBPF side
```

A trace containing spans from *both* `ebpf-capstone-fastapi` and
`ebpf-capstone-quarkus`, next to an `ebpf_capstone_*` series for the same window,
is the proof the whole stack — apps, backend, correlation, and eBPF — is working
as one.

## What you learned

- A single `curl` with a **`traceparent`** produces one **distributed trace**
  across **FastAPI → Quarkus**, with spans (Tempo), metrics (Prometheus), and
  logs (Loki) all sharing the trace_id — the full application-observability
  story you assembled in Chapter 62.
- An **Aya program adds the fourth view**: the kernel-side truth (syscalls,
  bytes, on-CPU time) for the same request, joined to the trace by **L7
  traceparent extraction** (OBI/uprobe) or by **time + pid + service**.
- Read end to end, one request now spans **application intent, application
  state, and kernel reality** in a single Grafana investigation — the reason
  eBPF and OpenTelemetry are worth combining, and the payoff of the whole book.

Next, Part 10 is an optional **field guide** to the command-line tools we've
leaned on for validation — `bpftrace`, `bpftool`, and the BCC tools — driven
from Python. Then a closing retrospective.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that both containers export OTLP to the Chapter
3 stack and the `traceparent` yields one trace with spans from both services;
that the Aya observer attaches and emits `ebpf_capstone_*`; that the response
trace_id lets you correlate the eBPF window with the trace; and treat the L7
`traceparent`-extraction reference as canonical-but-unverified while the
time/pid/service path is what the runnable observer uses.*
