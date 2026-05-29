---
title: "The observability stack"
order: 3
part: Foundations
description: Stand up Grafana + Tempo + Mimir + Loki in one Podman container, run a Python 3.14 OTLP client, and learn how your Rust eBPF programs will report their measurements into it.
duration: 20 minutes
---

An eBPF program that counts something is useless until you can *see*
the count. This chapter stands up the backend that every later chapter
reports into: Grafana for visualization, Tempo for traces, Mimir for
metrics, and Loki for logs — all in a single container — plus a Python
3.14 client that proves the pipe works. By the end you'll understand
exactly how a Rust user-space program turns "the kernel saw 4,812
`openat` calls" into a line on a Grafana panel.

The example lives in `examples/03-observability-stack/`.

## One container for the whole backend

Running Grafana, a trace store, a metric store, and a log store
separately is a lot of YAML. The `grafana/otel-lgtm` image packages all
of it — Grafana, **Tempo** (traces), **Mimir** (metrics), **Loki**
(logs), Prometheus, Pyroscope (profiles), and an OpenTelemetry
Collector — into one image with the datasources already wired together.
It is explicitly a development/demo backend, which is exactly our use
case. Recent versions even bundle **OBI** (OpenTelemetry eBPF
instrumentation) — eBPF that auto-generates HTTP/gRPC traces — which is
a fun thing to compare your hand-written probes against later.

Bring it up:

```bash
cd examples/03-observability-stack && ./demo.sh up
```

`demo.sh up` runs `podman compose -f compose.yaml up -d` and waits for
Grafana's health endpoint. Open **http://127.0.0.1:3000** — anonymous
admin is enabled, so there's no login. The compose file publishes three
ports on `127.0.0.1` only:

| Port | Purpose |
|------|---------|
| 3000 | Grafana UI |
| 4317 | OTLP gRPC ingest |
| 4318 | OTLP HTTP ingest |

A few Fedora-specific details are baked into `compose.yaml`, all
carried over from hard-won Podman experience: the image name is
**fully qualified** (`docker.io/grafana/otel-lgtm:0.28.0`) because the
bare short name won't resolve under Fedora's registry policy; Grafana
runs as **`user: "0"` with a `tmpfs` `/tmp`** because named volumes are
root-owned under rootless Podman; and the dashboard bind mount carries
**`:Z`** for SELinux.

## Prove the pipe with a Python 3.14 client

Before any Rust, confirm telemetry actually flows. The client
(`client/client.py`) is a tiny OTel program: each iteration it emits a
counter metric (`ebpf_events_total`), opens a trace span, and logs a
line tagged with the trace ID. It runs in **Podman** as Python 3.14 on
a UBI 9 image — clients always run in Podman, never on the host:

```bash
cd examples/03-observability-stack && ./demo.sh client
```

The one Podman networking detail worth internalizing: a rootless
container reaches a port published on the host via the special name
**`host.containers.internal`**. That's why the client exports to
`http://host.containers.internal:4318` while you, on the host, would
use `http://127.0.0.1:4318`.

After it runs, open the **eBPF with Aya — Overview** dashboard
(`http://127.0.0.1:3000/d/ebpf-overview`). You should see the event
rate climb and the log panel fill. If the metric is slow to appear,
give Mimir a few seconds to ingest and refresh — telemetry pipelines
are eventually-consistent.

## How Rust eBPF programs report in

Here is the mental model the rest of the tutorial relies on. An Aya
program has two halves:

- The **eBPF half** runs in the (guest) kernel. It can't talk to
  Grafana — it has no sockets, no allocator, no OTLP. What it *can* do
  is put numbers into **maps** (a counter per CPU, a histogram of
  latencies) and emit structured log records via **`aya-log`**.
- The **user-space half** runs as an ordinary Rust binary on the guest.
  It reads those maps on a timer, reads the log records, and exports
  them as OpenTelemetry metrics, traces, and logs using the
  `opentelemetry` + `opentelemetry-otlp` crates — pointed at the
  stack's OTLP endpoint.

So the data path for, say, `opensnoop` is:

```text
kernel probe on openat ──> per-CPU map (count, latency)
                              │
            user space reads map every 1s
                              │
        opentelemetry-otlp ──> OTLP/HTTP :4318 ──> Mimir/Tempo/Loki ──> Grafana
```

From the **target VM**, "the stack's OTLP endpoint" is the host's
address on the libvirt network (the `192.168.x.1` gateway), not
`127.0.0.1` — the stack runs on your laptop, the program runs in the
guest. Each networking chapter's `demo.sh` resolves that address for
you; when running by hand you pass it as
`OTEL_EXPORTER_OTLP_ENDPOINT`.

> **Why OTLP and not "just Prometheus"?** Prometheus scrapes; it needs
> to reach *into* the target on a schedule. OTLP pushes; the target
> reaches *out* to the stack. Push is far simpler across the
> host/guest boundary (no inbound firewall rules on the guest, works
> identically whether the program runs in a VM, a container, or bare
> metal). It also gives us traces and logs on the same wire, which is
> what makes the three-signal correlation in Grafana possible.

## The three signals, and why eBPF touches all of them

Most eBPF tutorials only ever print counters. Wiring into Tempo and
Loki as well as Mimir lets you tell a *story*:

- **Metric** (Mimir): `tcp_connect_latency` p99 spiked at 14:32.
- **Trace** (Tempo): the slow connections all came from one client
  span; the span carries the eBPF-measured kernel latency as an
  attribute.
- **Log** (Loki): `aya-log` shows the kernel saw `ECONNREFUSED` bursts
  from that peer at 14:32.

You don't need all three for every chapter — a simple `execsnoop` is
happy with logs alone — but the stack supports all three so you can
reach for whichever the program's story needs.

## Leave it running

Keep the stack up while you work through the tutorial; it's cheap.
When you're done for the day:

```bash
cd examples/03-observability-stack && ./demo.sh down
```

## What you should have now

- [x] `grafana/otel-lgtm` running, Grafana reachable at
  `127.0.0.1:3000`
- [x] The Python 3.14 client's metrics, traces, and logs visible in
  Grafana
- [x] A clear picture of the kernel → map → user space → OTLP →
  Grafana data path

[Next: Chapter 4 — The Rust + Aya toolchain →]({{ "/docs/04-rust-aya-toolchain/" | relative_url }})

---

*Verification status: <span class="status status--unverified">unverified</span>.
The stack, client, and dashboard have not yet been run end-to-end on
Fedora 44. The `otel-lgtm` tag (`0.28.0`) was current at authoring;
confirm and pin whatever you actually run.*
