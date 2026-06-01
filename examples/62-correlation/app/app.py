"""A tiny instrumented FastAPI service emitting all three signals on one
trace_id: a span (Tempo), a request counter (Prometheus), and a log line (Loki).
OTLP/HTTP → the Chapter 3 otel-lgtm collector."""
import logging
import os
import time

from fastapi import FastAPI
from opentelemetry import metrics, trace
from opentelemetry.exporter.otlp.proto.http.metric_exporter import OTLPMetricExporter
from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter
from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor
from opentelemetry.instrumentation.logging import LoggingInstrumentor
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

ENDPOINT = os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT", "http://host.containers.internal:4318")
RES = Resource.create({"service.name": "ebpf-correlation-fastapi", "service.namespace": "ebpf-with-aya"})

tp = TracerProvider(resource=RES)
tp.add_span_processor(BatchSpanProcessor(OTLPSpanExporter(endpoint=f"{ENDPOINT}/v1/traces")))
trace.set_tracer_provider(tp)

reader = PeriodicExportingMetricReader(OTLPMetricExporter(endpoint=f"{ENDPOINT}/v1/metrics"), export_interval_millis=2000)
metrics.set_meter_provider(MeterProvider(resource=RES, metric_readers=[reader]))

# LoggingInstrumentor stamps each log record with otelTraceID so Loki can link to Tempo
LoggingInstrumentor().instrument(set_logging_format=True)
log = logging.getLogger("work")
log.setLevel(logging.INFO)

tracer = trace.get_tracer("ebpf-correlation")
requests = metrics.get_meter("ebpf-correlation").create_counter("app_requests_total")

app = FastAPI()
FastAPIInstrumentor.instrument_app(app)  # automatic server spans, continues incoming traceparent


@app.get("/work")
def work():
    with tracer.start_as_current_span("work") as span:
        ctx = span.get_span_context()
        tid = format(ctx.trace_id, "032x")
        time.sleep(0.05)                       # pretend to do something
        requests.add(1, {"endpoint": "/work"})
        log.info("handled /work")              # log line carries otelTraceID
        return {"ok": True, "trace_id": tid}
