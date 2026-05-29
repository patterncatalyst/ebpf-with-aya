#!/usr/bin/env python3.14
"""
client.py — a stand-in load/telemetry client for the observability stack.

Later chapters replace the body of `do_work()` with something that actually
drives the target VM (hits a server on the peer, opens files to trip
opensnoop, spawns processes to trip execsnoop, etc.). Here it just proves the
OTLP pipe: it emits a metric, a trace span, and a log line on each iteration,
all of which land in Grafana via otel-lgtm.

Env:
  OTEL_EXPORTER_OTLP_ENDPOINT   default http://127.0.0.1:4318
  OTEL_SERVICE_NAME             default ebpf-client
  ITERATIONS                    default 60   (0 = run forever)
  INTERVAL_SECONDS              default 1.0
"""
import os
import time
import logging

from opentelemetry import trace, metrics
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter
from opentelemetry.exporter.otlp.proto.http.metric_exporter import OTLPMetricExporter

ENDPOINT = os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT", "http://127.0.0.1:4318")
SERVICE = os.environ.get("OTEL_SERVICE_NAME", "ebpf-client")
ITERATIONS = int(os.environ.get("ITERATIONS", "60"))
INTERVAL = float(os.environ.get("INTERVAL_SECONDS", "1.0"))

resource = Resource.create({"service.name": SERVICE, "service.namespace": "ebpf-with-aya"})

trace.set_tracer_provider(TracerProvider(resource=resource))
trace.get_tracer_provider().add_span_processor(
    BatchSpanProcessor(OTLPSpanExporter(endpoint=f"{ENDPOINT}/v1/traces"))
)
tracer = trace.get_tracer(SERVICE)

reader = PeriodicExportingMetricReader(
    OTLPMetricExporter(endpoint=f"{ENDPOINT}/v1/metrics"), export_interval_millis=2000
)
metrics.set_meter_provider(MeterProvider(resource=resource, metric_readers=[reader]))
meter = metrics.get_meter(SERVICE)
events = meter.create_counter("ebpf_events_total", description="Synthetic events from the client")

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
log = logging.getLogger(SERVICE)


def do_work(i: int) -> None:
    with tracer.start_as_current_span("client.iteration") as span:
        span.set_attribute("iteration", i)
        ctx = span.get_span_context()
        events.add(1, {"program": "client"})
        # Logback-style trace correlation tag so Loki's derivedField can link back.
        log.info("iteration %d done trace_id=%032x", i, ctx.trace_id)
        time.sleep(INTERVAL)


def main() -> None:
    log.info("client starting; exporting OTLP to %s as service.name=%s", ENDPOINT, SERVICE)
    i = 0
    while ITERATIONS == 0 or i < ITERATIONS:
        do_work(i)
        i += 1
    log.info("client done after %d iterations", i)


if __name__ == "__main__":
    main()
