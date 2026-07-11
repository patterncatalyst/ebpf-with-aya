---
title: "Appendix: eBPF metrics with Performance Co-Pilot (PCP)"
order: 69
part: Addenda
description: "Performance Co-Pilot is the other major metrics model — pull-and-record rather than push-and-stream, host-centric, decades old, and shipped in Fedora. It has a first-class eBPF on-ramp: pmdabpf runs BPF CO-RE modules and pmdabpftrace runs bpftrace scripts, both surfaced as ordinary PCP metrics with archives, alerting, and a Grafana plugin. This appendix covers what PCP is, how it's used and extended, and how it meets eBPF — and how it complements the OTLP stack the rest of the book uses."
duration: 35 minutes
---

The whole book pushes metrics: an Aya loader exports OTLP to a collector, and
Grafana reads it. **Performance Co-Pilot (PCP)** is the other major model in Linux
performance monitoring, and it works the other way around — a daemon on each host
*collects* metrics from pluggable agents, clients *pull* from that daemon, and a
logger *records* everything to disk for later replay. It is decades old (born at
SGI, now maintained by Red Hat), ships in Fedora, and matters here for one
reason: it has a first-class eBPF on-ramp, and it lands in the same Grafana you
already run. This appendix is a field guide to PCP and the seam where it meets
eBPF.

{% include excalidraw.html
   file="pcp-ebpf"
   alt="Performance Co-Pilot architecture and its eBPF intersection. On the left, pluggable agents called PMDAs feed a collector daemon, pmcd: pmdalinux (kernel, /proc, cgroups); pmdabpf (eBPF CO-RE ELF modules); pmdabpftrace (bpftrace scripts); and pmdaopenmetrics (scrapes an Aya loader's Prometheus /metrics endpoint). pmdabpf and pmdabpftrace are highlighted as the eBPF intersection, and pmdaopenmetrics bridges the book's Aya loaders. On the right, consumers read from pmcd: pminfo, pmrep, pmstat, and pmchart for live query and reporting; pmlogger to record archives for later replay; pmie, the inference engine, for alerts; and pmproxy with pmseries for a REST API and time series stored in Redis. Below that, grafana-pcp brings PCP metrics into the same Grafana dashboards as the OTel stack. The model is pull-and-record — archives and alerts — which complements the book's push-based OTLP stream; both land in the same Grafana."
   caption="Figure 69.1 — PCP collects from PMDAs into pmcd; pmdabpf and pmdabpftrace are the eBPF on-ramp" %}

## What PCP is

PCP is a toolkit, not a single program, built in the Unix tradition of small
components that compose. At the centre of each host runs **pmcd**, the
Performance Metrics Collector Daemon. pmcd itself knows nothing about any
particular metric; it delegates to **PMDAs** — Performance Metrics Domain Agents,
one per "domain" of metrics. `pmdalinux` exposes the kernel's `/proc` and cgroup
counters; other PMDAs cover databases, web servers, GPUs, and — the point of this
appendix — eBPF.

What sets PCP apart from a bag of counters is that every metric carries rich,
self-describing metadata. Each has a place in the **Performance Metrics Name
Space** (PMNS) — a dotted hierarchy like `kernel.all.load` or `bpf.runqlat` — a
unique identifier (PMID), a data type, **units** and a **semantic** (is this a
counter, an instantaneous gauge, a discrete value?), and where it has multiple
instances (per-CPU, per-disk, per-process) an **instance domain** describing them.
A client never has to guess what a number means or how to rate-convert it; the
metadata says. And because pmcd speaks a network protocol, a client on one machine
can read metrics from pmcd on another — monitoring is local or remote with the
same tools.

## How you use it

The everyday tools all talk to pmcd:

- **`pminfo`** — the metadata-and-values workhorse: `pminfo -f kernel.all.load`
  fetches a metric with its description and current value; `pminfo -t` shows its
  help text.
- **`pmrep`**, **`pmstat`**, **`pmval`** — tabular reporting and sampling over
  time, the command-line equivalents of a dashboard row.
- **`pmchart`** and **`pcp atop`** — graphical and `top`-like live views.

The feature that distinguishes PCP from most metrics systems is **`pmlogger`**:
it records selected metrics to an on-disk **archive**, and every one of the tools
above can replay an archive instead of reading live — `pmrep --archive
20260601 kernel.all.load` reports last week's load exactly as if you were there.
That is retrospective analysis: when something broke at 03:00, you open the
archive and look, rather than wishing you'd been watching. Rounding out the set,
**`pmie`** is an inference engine that evaluates rules over metrics and fires
alerts, and **`pmproxy`** exposes a REST API plus **`pmseries`**, a Redis-backed
time-series interface. A minimal bring-up is two commands:

```bash
[vm]$ sudo dnf install -y pcp pcp-zeroconf
[vm]$ sudo systemctl enable --now pmcd pmlogger
[vm]$ pminfo -f kernel.all.load          # it's already collecting
```

## What it's useful for

PCP's sweet spot is **always-on, low-overhead, whole-system recording**. Because
pmlogger archives run continuously and cheaply, PCP is the flight recorder for a
fleet: you get retrospective root-cause ("what did the run queue look like during
the incident?") without having had a tracer attached at the time. It is host- and
system-centric rather than request-centric, it scales across machines through
pmcd and `pmlogger` farms, and it complements ad-hoc tracing rather than replacing
it — you reach for `bpftrace` to investigate a hypothesis, and for PCP to have
been recording the evidence all along.

## How you extend it

The extension seam is the PMDA. Anything you can express as a PMDA becomes a set
of first-class PCP metrics — archived, alertable, and graphable like any other.
PMDAs can be written in C for speed, or in Python or Perl for convenience, and
the interface is small: describe your metrics' metadata and answer "fetch"
requests for their values. You don't always have to write one, though:
**`pmdaopenmetrics`** scrapes any Prometheus/OpenMetrics HTTP endpoint and turns
those metrics into PCP metrics. That single agent is the bridge that lets PCP
ingest things it was never written for — including, as we'll see, the `ebpf_*`
metrics this book's loaders already export.

## eBPF with PCP

This is the intersection. PCP has three ways to source metrics from eBPF, plus a
bridge for your own Aya tools.

**`pmdabpf` — BPF CO-RE modules as metrics.** The modern agent (`pcp-pmda-bpf`)
loads eBPF programs built with libbpf and BTF (the same CO-RE approach as Chapter
58) and exposes their output as PCP metrics. It reads an ini-style config at
`$PCP_PMDAS_DIR/bpf/bpf.conf` (default `/var/lib/pcp/pmdas/bpf/bpf.conf`), one
`[section]` per module, each with an `enabled` flag; the compiled module objects
live under `.../bpf/modules/`. Enable a couple and install the agent:

```bash
[vm]$ sudo dnf install -y pcp-pmda-bpf
[vm]$ sudo sed -n 's/^\[\(.*\)\]/module: \1/p' /var/lib/pcp/pmdas/bpf/bpf.conf   # list modules
# enable e.g. runqlat and biolatency by setting enabled=true in their sections, then:
[vm]$ cd /var/lib/pcp/pmdas/bpf && sudo ./Install
[vm]$ pminfo -f bpf                       # the eBPF metrics now live under the bpf.* tree
```

`pmdabpf` is launched by pmcd and never run directly; the `./Install` script
registers it. From then on `bpf.runqlat`, `bpf.biolatency`, and friends are just
PCP metrics — `pmrep`, `pmlogger`, `pmie`, and Grafana treat them like any other.

**`pmdabpftrace` — bpftrace scripts as metrics.** The bpftrace PMDA
(`pcp-pmda-bpftrace`) runs bpftrace scripts and exports their maps as PCP metrics,
including histograms. Scripts can be started on demand, or placed (root-writable,
for safety) in an autostart directory so they run from PMDA startup — the
production-safe pattern. With the Grafana plugin this becomes on-demand live eBPF
analysis from a dashboard.

**`pmdabcc` — the predecessor.** Before CO-RE, the BCC PMDA ran BPF through the
BCC Python frontend (runtime-compiled with Clang, needing kernel headers). It
still exists and works, but `pmdabpf` is the direction of travel for the same
reasons Chapter 66 gave: precompiled CO-RE modules avoid shipping a compiler to
every host.

**Bridging your own Aya tools into PCP.** Everything in this book exports `ebpf_*`
metrics over OTLP. Two paths bring those into PCP. The direct one: point
`pmdaopenmetrics` at a Prometheus endpoint that carries them — either the
otel-lgtm stack's bundled Prometheus, or a loader extended to expose a
`/metrics` endpoint — by dropping a `.url` file in the OpenMetrics PMDA's config
directory, after which your `ebpf_he_op_latency_seconds` (Chapter 68) or
`ebpf_runqlat` lands in the `openmetrics.*` tree, gets archived by pmlogger, and
can trigger a pmie alert. The thorough one: write a small Python PMDA that reads
your Aya program's map or output directly and presents it with proper PCP
metadata. Either way your bespoke eBPF metric gains PCP's archive and alerting for
free.

**Grafana ties it together.** The **grafana-pcp** plugin adds PCP as a Grafana
data source — live through pmproxy, historical through pmseries/Redis. Install it
and your `bpf.*` and `openmetrics.*` metrics appear on dashboards alongside the
OTel `ebpf_*` metrics the rest of the book produces, in the same Grafana.

## PCP or the book's stack?

They are different philosophies, and the answer is "both, for different
jobs." The book's approach — a bespoke Aya program
pushing OTLP — wins when you need a *custom* probe and request-level correlation:
your own metric, joined to traces and logs on a `trace_id`, as in the Chapter 63
capstone. PCP wins when you want *standard* eBPF tooling running 24/7 with
low overhead, recorded to archives for retrospective analysis and wired to alerts,
without writing or operating a loader at all — enable a `pmdabpf` module and it is
collecting forever. The two interoperate cleanly through grafana-pcp and
`pmdaopenmetrics`, so the choice is never exclusive: write Aya where you need
bespoke insight, lean on PCP where you need durable, always-on host recording, and
read both in one Grafana.

Try the agent and the bridge:

```bash
cd examples/69-pcp-ebpf && ./demo.sh
```

`examples/69-pcp-ebpf/demo.sh` installs PCP and the BPF PMDA on the VM, enables a
module, shows the metrics under `bpf.*` with `pminfo`/`pmrep`, records and replays
a short pmlogger archive, and drops a `pmdaopenmetrics` `.url` file that bridges an
`ebpf_*` Prometheus endpoint into PCP; `examples/69-pcp-ebpf/README.md` has the
details and the grafana-pcp data-source steps.

## What you learned

- **PCP** is a pull-and-record metrics toolkit: PMDAs feed the `pmcd` collector,
  clients (`pminfo`, `pmrep`, `pmchart`) read it live, `pmlogger` archives it for
  retrospective replay, `pmie` alerts on it, and `pmproxy`/`pmseries` expose REST
  and time series. Every metric is self-describing (PMNS, units, semantics).
- Its strength is **always-on, low-overhead, system-wide recording** — the flight
  recorder that complements the hypothesis-driven tracing you do with `bpftrace`.
- It is **extended through PMDAs** (C/Python/Perl), and `pmdaopenmetrics` ingests
  any Prometheus/OpenMetrics endpoint without writing one.
- Its **eBPF on-ramp** is `pmdabpf` (BPF CO-RE modules as metrics) and
  `pmdabpftrace` (bpftrace scripts as metrics), with the legacy `pmdabcc` behind
  them; your own Aya `ebpf_*` metrics bridge in via `pmdaopenmetrics`, and
  **grafana-pcp** shows all of it next to the book's OTLP metrics in one Grafana.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
