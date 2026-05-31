# 46 · Capstone — tying the three signals together

Turn one request to a **Java** and a **Python** service into three correlated
OpenTelemetry signals — a **span** (Tempo), a **log** (Loki), and **RED
metrics** (Prometheus) sharing one `trace_id` — using a kernel **socket** probe,
because JIT/interpreted runtimes have no app symbols to hook. A teaching-grade
sketch of the OpenTelemetry eBPF Instrumentation project (**OBI**).

## Pieces

- `httpwatch-ebpf` — kprobes on `tcp_recvmsg`/`tcp_sendmsg`, keyed by the
  socket pointer; emits `Req { dur_ns, comm }` per request to a ring buffer.
- `httpwatch-common` — the shared `Req` record.
- `httpwatch` — mints a `trace_id` per request and emits span + log + metric.
- `services/java` (plain JDK HTTP server) and `services/python` (FastAPI),
  both containerized; neither uses an OTel SDK.

## Run it

```bash
./demo.sh          # build+run both services on $VM, drive load, attach the probe
./demo.sh build    # just build the probe on the host
```

## See the correlation (Grafana, 127.0.0.1:3000)

1. Metric: `histogram_quantile(0.95, sum by (le,service) (rate(ebpf_http_server_duration_ms_bucket[1m])))`
2. Tempo: the `http.server.request` spans, by `service.name`
3. Loki: `{service_name=~".*three-signals.*"}` — each line carries a `trace_id`

## The production tool

OBI does this for real — L7 parsing, W3C context propagation, TLS visibility,
DB instrumentation — zero code changes. See
<https://opentelemetry.io/docs/zero-code/obi/>.

## Verification status

**Unverified.** Confirm the `tcp_recvmsg`/`tcp_sendmsg` kprobe arg0 = `struct
sock *`; that one recv/send pair is a usable request proxy here; the
opentelemetry 0.27 traces/logs builder APIs; whether the metrics SDK emits
exemplars (metric→trace may be absent); and end-to-end Tempo/Loki/Prometheus
correlation.
