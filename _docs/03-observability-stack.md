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

The example lives in `examples/03-observability-stack/`. Its `./demo.sh`
brings the stack up with podman-compose; its `README.md` has the details.

{% include excalidraw.html
   file="obs-data-path"
   alt="Data path: an eBPF program writes to a map; the Aya loader reads the map and emits OTLP/HTTP metrics to the otel-lgtm stack, which Grafana visualizes."
   caption="Figure 3.1 — from kernel counter to Grafana panel" %}

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

So the data path for, say, `opensnoop` looks like this:

{% include excalidraw.html
   file="reports-in"
   alt="The opensnoop openat probe in the guest kernel writes counts and latency into a per-CPU map. The user-space loader reads the map and produces two faces of output: it prints a live table to your terminal (what chapters tell you to watch), and it pushes ebpf_* metrics via OTLP on port 4318 to the otel-lgtm stack, which stores them and charts them in Grafana on port 3000 over time. The program runs in the guest while the stack runs on your laptop, so the loader pushes out via OTLP."
   caption="Figure 3.2 — one probe, two faces of output: a live terminal table and ebpf_* metrics in Grafana" %}

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

### Where your output shows up

Every program chapter from here produces output in **two places**, and it's
worth fixing which is which now so later instructions are unambiguous:

- **Your terminal** — the loader prints a small live view (a table, a
  histogram, a per-CPU bar) to the terminal where you ran `demo.sh`. When a
  chapter says "watch the per-CPU busy percentages" or "watch the latency
  histogram fill in," *this* is what it means — the text scrolling in your
  terminal, on the host, fed back over SSH from the program running in the
  guest.
- **Grafana** — the same loader exports metrics named **`ebpf_*`** over OTLP.
  These land in the stack on your laptop and are what you chart over time. To
  see them, open Grafana at `127.0.0.1:3000`, go to **Explore**, pick the
  Prometheus data source (otel-lgtm bundles Prometheus-compatible storage
  alongside Grafana), and start a query with `ebpf_` — autocomplete lists
  every metric the chapters export. For `opensnoop` you'd find something like
  `ebpf_opensnoop_opens_total`; graph it and you have opens-per-second by
  process, live. Each chapter names the specific `ebpf_*` metric to look for
  under its "Build, deploy, observe" step.

The rule of thumb: the **terminal** view is for the quick "is it working right
now" glance while you run the demo; **Grafana** is where the same numbers
become a time series you can watch, compare across runs, and correlate with
the traces and logs on the same wire.


If you'd rather click than type, the stack auto-loads a dashboard the moment
it comes up — **eBPF with Aya — Overview**, in the *eBPF with Aya* folder in
Grafana. It has a **Metric (explorer)** picker over every `ebpf_*` series (a
rate panel for `*_total` counters, a raw panel for gauges) alongside the
events and logs streams — a single front door for any chapter's output. Each
chapter still names the precise query, but this is the quickest way to browse.

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

Keep the stack up while you work through the tutorial; it's cheap — and it's
not scenery. **Every program chapter that follows exports its `ebpf_*`
metrics to this stack**, so the demo you run in Chapter 6, the latency
histograms in the tracing chapters, the per-CPU bars in the schedulers
chapters, and the request timings in the application chapters all show up here
in Grafana with no further setup. That's the payoff for standing it up once
now. When you're done for the day:

```bash
cd examples/03-observability-stack && ./demo.sh down
```

## What you should have now

- [x] `grafana/otel-lgtm` running, Grafana reachable at
  `127.0.0.1:3000`
- [x] The Python 3.14 client's metrics, traces, and logs visible in
  Grafana
- [x] A clear picture of the two faces of output — a live table in your
  terminal and `ebpf_*` metrics charted in Grafana at `127.0.0.1:3000`

[Next: Chapter 4 — The Rust + Aya toolchain →]({{ "/docs/04-rust-aya-toolchain/" | relative_url }})

---

*Verification status: <span class="status status--unverified">unverified</span>.
The stack, client, and dashboard have not yet been run end-to-end on
Fedora 44. The `otel-lgtm` tag (`0.28.0`) was current at authoring;
confirm and pin whatever you actually run.*
