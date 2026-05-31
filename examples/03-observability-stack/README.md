# Example 03 — The observability stack

The Grafana / Tempo / Mimir / Loki backend that every later chapter
exports into, plus a Python 3.14 OTLP client that proves the pipe.

## What this shows

- `grafana/otel-lgtm:0.28.0` — one container bundling Grafana, Tempo
  (traces), Mimir (metrics), Loki (logs), Prometheus, Pyroscope
  (profiles), and an OpenTelemetry Collector. This is the
  Grafana/Tempo/Mimir stack the tutorial targets.
- A **Python 3.14** client (`client/client.py`) running in **Podman**
  that emits a counter metric (`ebpf_events_total`), a trace span per
  iteration, and a correlated log line — all over OTLP/HTTP.
- A provisioned dashboard (`eBPF with Aya — Overview`) showing event
  rate, total events, and program logs.

## Run it

```bash
./demo.sh            # up + build/run client + verify + print URLs
./demo.sh up         # just the stack
./demo.sh client     # just (re)build and run the client
./demo.sh down        # tear everything down
```

Then open **http://127.0.0.1:3000** (anonymous admin, no login) and
look at the `eBPF with Aya — Overview` dashboard, or use Explore.

## How later chapters use this

| Signal | Source in later chapters | Where it shows in Grafana |
|--------|--------------------------|---------------------------|
| Metrics | Aya user space exports counters/histograms (events seen, latency) via OTLP | Mimir/Prometheus panels |
| Traces | User space wraps a load operation in a span; the eBPF program's measurement attaches as attributes | Tempo |
| Logs | `aya-log` messages from the kernel program, forwarded by user space | Loki |

The Rust user-space side uses the `opentelemetry` + `opentelemetry-otlp`
crates, pointed at `http://127.0.0.1:4318` (or, from inside the target
VM, at the host's IP on the libvirt network). Chapter 6 wires the first
program's output through here end to end.

## Ports

| Service | Host port |
|---------|-----------|
| Grafana UI | 127.0.0.1:3000 |
| OTLP gRPC | 127.0.0.1:4317 |
| OTLP HTTP | 127.0.0.1:4318 |

## Notes

- The image name is **fully qualified** (`docker.io/grafana/otel-lgtm`)
  — the bare short name doesn't resolve under Fedora's Podman registry
  policy.
- Grafana runs as `user: "0"` with `tmpfs` for `/tmp` because named
  volumes are root-owned under rootless Podman.
- `otel-lgtm` already provisions correlated datasources; the
  `grafana/provisioning/datasources/datasources.yaml` here is optional
  reference for customising the three-signal correlation.

*Status: unverified — not yet run end-to-end on Fedora 44. See the
reconciliation plan.*

## The provisioned dashboard

`grafana/provisioning/dashboards/ebpf-overview.json` is mounted into the stack
and loads automatically — open Grafana and find **eBPF with Aya — Overview**
in the *eBPF with Aya* folder. Alongside the events and logs panels it has a
**Metric (explorer)** variable listing every `ebpf_*` series the tutorial
exports, feeding a rate panel (for `*_total` counters) and a raw panel (for
gauges) — one front door for any chapter's output. Each chapter also names the
exact Explore query.
