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

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab VM:
the `tcp_recvmsg`/`tcp_sendmsg` kprobes build, load, and attach, and the pipeline
emits correlated spans (Tempo), logs (Loki), and RED metrics (Prometheus) sharing
a `trace_id`. The kprobe arg0 = `struct sock *` assumption and the one recv/send
pair as a request proxy are kernel- and workload-specific. Note the candid seam:
this is intra-service correlation, not distributed tracing, and exemplar support
(metric→trace) in the Rust metrics SDK may be absent on a given run.
