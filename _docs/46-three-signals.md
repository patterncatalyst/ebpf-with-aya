---
title: "Capstone: tying the three signals together"
order: 46
part: Application targets
description: "A short capstone for the applications part. We can't symbol-probe Java or Python the way we did nginx — so we instrument at the kernel socket layer, the way the OpenTelemetry eBPF Instrumentation (OBI) project does, and turn one request into three correlated signals: a span, a log, and a RED metric sharing a trace_id. Then we watch the click-through correlation in Grafana and place it against the production tool."
duration: 50 minutes
---

This part has aimed eBPF at one real server at a time. This capstone steps
back and asks the question the whole observability stack was built for: can we
make eBPF produce the **three signals** — metrics, logs, and traces —
*correlated*, so that a latency spike on a graph is one click from the trace
that caused it and the logs that explain it? And can we do it for runtimes we
*can't* symbol-probe — Java and Python — the way the real world does?

The answer is yes, and the route to it is the central lesson of this part. In
Chapter 45 we attached uprobes to nginx's own functions because nginx is a
native binary with symbols. **Java is JIT-compiled and Python is interpreted**
— there is no stable `handle_request` symbol to hook in either. So we drop to
the one layer every networked service shares no matter the language: the
**kernel socket layer**. That is exactly the insight behind the OpenTelemetry
eBPF Instrumentation project (**OBI**, the donated Grafana Beyla), and this
chapter builds a teaching-grade version of it, then points you at the real one.

The code is in `examples/46-three-signals/`. `./demo.sh` there runs a Java and
a Python service, drives load, and attaches the probe; its `README.md` has the
details.

{% include excalidraw.html
   file="three-signals"
   alt="One HTTP request to a Java or Python service is seen by an eBPF socket probe that times recv to send and captures the process comm. The loader mints one trace_id and emits three correlated signals: a span to Tempo (http.server.request), a log to Loki carrying the trace_id, and a RED metric to Prometheus with an exemplar. All three share the trace_id, so in Grafana you can click from metric to trace to logs. We mint the trace_id in the loader, while OBI propagates real W3C context across services."
   caption="Figure 46.1 — One request, one trace_id, three correlated signals" %}

## Why the socket layer

A uprobe needs a symbol or an offset in a binary. That works for C servers and
even for Rust and Go (Chapters 14–15), but a JVM compiles methods to machine
code *at runtime* at addresses that don't exist on disk, and CPython runs your
handler as bytecode inside the interpreter loop — neither presents a stable
"handle one request" function to attach to. What they *do* share is that every
request arrives and departs as bytes through a socket: `recv` then `send` on
the kernel's TCP path. Hook there and you get a language-agnostic view of
"a request was handled, and it took this long" without one line of app change.
That is precisely how OBI achieves Java/.NET/Go/Python/Ruby/Node/C/C++/Rust
coverage from a single agent.

## How the code works

The kernel side hooks two TCP functions and pairs them per socket:

```rust
#[map] static STARTS: HashMap<u64, u64> = HashMap::with_max_entries(10240, 0); // sock ptr -> recv ts
#[map] static EVENTS: RingBuf = RingBuf::with_byte_size(1 << 16, 0);

#[kprobe] // tcp_recvmsg(struct sock *sk, ...)
pub fn on_recv(ctx: ProbeContext) -> u32 {
    let sk: u64 = ctx.arg(0).unwrap_or(0);
    if sk != 0 { let _ = STARTS.insert(&sk, &unsafe { bpf_ktime_get_ns() }, 0); }
    0
}

#[kprobe] // tcp_sendmsg(struct sock *sk, ...)
pub fn on_send(ctx: ProbeContext) -> u32 {
    let sk: u64 = ctx.arg(0).unwrap_or(0);
    if let Some(&start) = unsafe { STARTS.get(&sk) } {
        let now = unsafe { bpf_ktime_get_ns() };
        let _ = STARTS.remove(&sk);
        if let Some(mut e) = EVENTS.reserve::<Req>(0) {
            let comm = bpf_get_current_comm().unwrap_or_default();
            unsafe { (*e.as_mut_ptr()) = Req { dur_ns: now - start, comm }; }
            e.submit(0);
        }
    }
    0
}
```

The key choices:

- We key the request on the **`struct sock *` pointer** (read as a plain
  `u64`, never dereferenced — like the request pointer in Chapter 45). A
  `recv` stamps the start; the following `send` on the same socket closes it
  and emits the duration.
- **`bpf_get_current_comm()`** tags each event with the process name —
  `java`, `python3.14` — so user space can attribute it to the right service
  *without* reading any kernel struct. That's our stand-in for OBI's far
  richer service discovery.
- This is deliberately a **simplification**: one `recv`/`send` pair is a
  decent proxy for a request on a simple keep-alive-light workload, but it
  doesn't parse HTTP, doesn't handle pipelining, and times service work rather
  than the full request. OBI parses the L7 protocol to get this right; we're
  after the *shape*, not production fidelity.

### The part that matters: three signals, one trace_id

The user side is where the capstone earns its name. For each event it mints
**one trace_id** and emits all three signals against it:

```rust
let trace_id = TraceId::from_bytes(rand::random());
// 1. a span (-> Tempo), timed to the measured interval
let span = tracer.span_builder("http.server.request")
    .with_trace_id(trace_id)
    .with_start_time(now - Duration::from_nanos(e.dur_ns))
    .with_end_time(now)
    .with_attributes([KeyValue::new("service.name", svc_from(&e.comm))])
    .start(&tracer);
span.end();
// 2. a log (-> Loki), carrying the same trace_id for correlation
logger.emit(LogRecord::builder()
    .with_trace_context(trace_id, span_id)
    .with_body(format!("handled request in {} µs", e.dur_ns / 1000))
    .build());
// 3. RED metrics (-> Prometheus): a request count and a duration histogram
requests.add(1, &[KeyValue::new("service", svc)]);
duration_ms.record(e.dur_ns as f64 / 1e6, &[KeyValue::new("service", svc)]); // exemplar carries trace_id
```

The `trace_id` is the thread that stitches the signals together. Tempo stores
the span under it; the log carries it (Loki's `derivedFields` turns it into a
link); and where the metrics SDK supports **exemplars**, the histogram sample
carries it too, so a point on the latency graph links straight to its trace.
That wiring — trace→logs, trace→metrics, exemplars — is exactly what the
observability stack's datasources were provisioned for back in Chapter 3;
this is the chapter that finally exercises all of it.

> **Honest note on the seams.** Minting the `trace_id` in the loader gives us
> *intra-service* correlation (this request's span, log, and metric line up),
> which is the teachable goal. It is **not** distributed tracing: we don't
> read the caller's W3C `traceparent`, so kernel spans aren't children of an
> upstream request's trace. And **exemplar** support in the Rust metrics SDK
> has lagged the spec, so that one hop (metric→trace) is the most likely to
> be missing on your run. Both are exactly the problems OBI solves properly.

## Build, deploy, observe

```bash
cd examples/46-three-signals && ./demo.sh
```

The demo builds and runs two containers — a **Java** HTTP service and a
**Python/FastAPI** service — drives a mix of requests at both, and attaches
the socket probe. In the terminal you'll see a line per completed request
(`java 1840µs`, `python3.14 920µs`).

**In Grafana** (`127.0.0.1:3000`), this is the click-through to try:
- Explore the **metric** — `histogram_quantile(0.95, sum by (le,service) (rate(ebpf_http_server_duration_ms_bucket[1m])))` — and watch p95 per service.
- Open **Tempo** and find the `http.server.request` spans; each carries a `service.name`.
- Open **Loki** with `{service_name=~".*three-signals.*"}` and confirm each log line carries a `trace_id` that resolves to a span.

That metric → trace → logs path, on a Java and a Python service you never
modified, is the whole point of the OpenTelemetry-plus-eBPF story.

## The production tool: OBI

What we hand-rolled, the **OpenTelemetry eBPF Instrumentation** project does
for real and at scale. OBI auto-instruments HTTP/S and gRPC services with eBPF
— no code changes, no libraries, no restarts — emitting RED metrics and
distributed trace spans, enriching logs with trace context, and even seeing
inside TLS without decryption. It does real L7 protocol parsing, propagates
W3C context across services for true distributed traces, ships database
instrumentation (PostgreSQL, MySQL, Redis, and more), and runs either
standalone or as an OpenTelemetry Collector receiver.

{% include excalidraw.html
   file="obi-arch"
   alt="OBI architecture across two layers. In kernel space, kprobes on sockets (TCP recv/send) and uprobes on TLS/libssl and language runtimes capture events. In user space, the OBI agent maps those raw events to OpenTelemetry semantic conventions (RED metrics and spans) and feeds the OpenTelemetry Collector, which exports over OTLP to traces in Tempo, metrics in Prometheus, and logs in Loki. OBI is zero-code auto-instrumentation that turns eBPF probes into OTel signals with no app changes, and it is language-agnostic because it reads the wire, not your symbols."
   caption="Figure 46.2 — OBI: the production shape of what this chapter sketched" %}

Read it as the same picture as Figure 46.1, industrialized: the socket and
TLS probes are the `recv`/`send` hook done thoroughly; the agent is our loader
done with real protocol parsing and semantic conventions; the Collector and
backends are the stack you already run. If you take one thing from this
capstone, it's that "observe a service you didn't write, in any language, with
correlated signals" is not aspirational — it's a deployed OTel project, and
you now understand exactly how it works under the hood. See the OpenTelemetry
[OBI documentation](https://opentelemetry.io/docs/zero-code/obi/) to run it for
real.

## Cross-check

```bash
[vm]$ curl -s -o /dev/null -w '%{time_total}\n' http://127.0.0.1:8081/   # Java, client-side
[vm]$ curl -s -o /dev/null -w '%{time_total}\n' http://127.0.0.1:8082/   # Python, client-side
[vm]$ sudo bpftool prog show | grep -E 'kprobe'                          # the two attached kprobes
```

The client-side `time_total` should bracket the per-request durations the
probe reports for each service; the span durations in Tempo should match. When
all three agree, your hand-built three-signal pipeline is faithful.

## What you learned

- You **can't** symbol-probe Java (JIT) or Python (interpreted), so
  language-agnostic instrumentation hooks the **kernel socket layer** — the
  insight behind OBI.
- A measured interval becomes a **span**, a per-event record becomes a **log**,
  and counts become **RED metrics**; minting **one `trace_id`** across all
  three is what makes them correlate into the metric→trace→logs click-through.
- This exercises the trace/log/metric correlation the stack was provisioned
  for in Chapter 3 — and **OBI** is the production tool that does it properly,
  with L7 parsing, W3C propagation, and TLS visibility.

Next, Chapter 47 returns to a single hard target — a **postgres** database —
to observe queries and lock waits.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: the `tcp_recvmsg`/`tcp_sendmsg` kprobe
signatures and that `ctx.arg(0)` is the `struct sock *`; that a single
recv/send pair is a usable request proxy for these services; the
opentelemetry-rust 0.27 traces + logs builder APIs used here (`span_builder`,
`with_trace_id`, the logs `LogRecord` bridge); whether the metrics SDK emits
**exemplars** (the metric→trace hop may be absent); and that Tempo/Loki/
Prometheus correlation resolves end to end.*
