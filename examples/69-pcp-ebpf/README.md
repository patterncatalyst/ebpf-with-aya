# 69 — eBPF metrics with Performance Co-Pilot (PCP)

PCP is the pull-and-record counterpart to the push-based OTLP stack the rest of
the book uses: a `pmcd` daemon collects metrics from pluggable **PMDAs**, clients
pull from it, `pmlogger` records archives for retrospective replay, `pmie` alerts,
and **grafana-pcp** shows it all in Grafana. Its eBPF on-ramp is the BPF PMDA.

## The eBPF intersection

- **`pmdabpf`** (`pcp-pmda-bpf`) — loads eBPF **CO-RE** ELF modules (libbpf + BTF)
  and exposes them as `bpf.*` metrics (runqlat, biolatency, …). Config:
  `/var/lib/pcp/pmdas/bpf/bpf.conf` (one `[section]` per module, `enabled` flag);
  modules under `…/bpf/modules/`; register with `./Install`.
- **`pmdabpftrace`** (`pcp-pmda-bpftrace`) — runs bpftrace scripts as metrics
  (incl. histograms); autostart scripts (root-writable) for production.
- **`pmdabcc`** — the older BCC/Python (runtime-compiled) PMDA, superseded by the
  CO-RE `pmdabpf`.
- **Bridge your own Aya metrics** — `pmdaopenmetrics` scrapes a Prometheus/
  OpenMetrics endpoint into the `openmetrics.*` tree, so the book's `ebpf_*`
  metrics gain PCP archives + alerts. See `openmetrics-ebpf.url.example`.

## Run it

```bash
cd examples/69-pcp-ebpf && ./demo.sh
# bridge the book's metrics too:
PROM_URL=http://host.containers.internal:9090/metrics ./demo.sh
```

`demo.sh` (all steps on the VM): installs PCP + the BPF/bpftrace PMDAs +
grafana-pcp; enables `runqlat`/`biolatency` in `bpf.conf` and installs the agent;
shows the metrics with `pminfo bpf` and `pmrep`; records a 10-second `pmlogger`
archive and replays it; and, if `PROM_URL` is set, drops a `pmdaopenmetrics`
`.url` bridge for the book's `ebpf_*` metrics.

- **Live:** `pminfo -f bpf.runqlat`, `pmrep -t 1 bpf.runqlat`
- **Archive (retrospective):** `pmrep --archive <archive> bpf.runqlat`
- **Grafana:** install the `grafana-pcp` plugin, add a **PCP** data source
  (Vector for live via pmproxy, Series for historical via pmseries/Redis), then
  graph `bpf.*` and `openmetrics.*` next to the OTel `ebpf_*` metrics.

## Status

**Unverified** — written against PCP's documented behavior; not run on hardware.
Confirm on Fedora 44: package names/versions (`pcp`, `pcp-pmda-bpf`,
`pcp-pmda-bpftrace`, `pcp-pmda-openmetrics`, `grafana-pcp`); the `bpf.conf` module
section names and `./Install` flow; that enabled modules show under `pminfo bpf`;
and the OpenMetrics bridge against whichever Prometheus endpoint carries your
`ebpf_*` metrics. Install only from Fedora repositories.
