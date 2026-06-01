"""FastAPI /checkout — the trace's entry point. Calls Quarkus /inventory
(propagating the traceparent), emits a span + metric + trace-stamped log,
and returns the trace_id so the demo can correlate the eBPF window."""
import logging, os
from fastapi import FastAPI
import requests
from opentelemetry import metrics, trace
from opentelemetry.exporter.otlp.proto.http.metric_exporter import OTLPMetricExporter
from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter
from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor
from opentelemetry.instrumentation.logging import LoggingInstrumentor
from opentelemetry.instrumentation.requests import RequestsInstrumentor
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import PeriodicExportingMetricReader
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor

ENDPOINT = os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT", "http://host.containers.internal:4318")
INVENTORY = os.environ.get("INVENTORY_URL", "http://quarkus:8080/inventory")
RES = Resource.create({"service.name": "ebpf-capstone-fastapi", "service.namespace": "ebpf-with-aya"})

tp = TracerProvider(resource=RES)
tp.add_span_processor(BatchSpanProcessor(OTLPSpanExporter(endpoint=f"{ENDPOINT}/v1/traces")))
trace.set_tracer_provider(tp)
metrics.set_meter_provider(MeterProvider(resource=RES, metric_readers=[
    PeriodicExportingMetricReader(OTLPMetricExporter(endpoint=f"{ENDPOINT}/v1/metrics"), export_interval_millis=2000)]))
LoggingInstrumentor().instrument(set_logging_format=True)
RequestsInstrumentor().instrument()   # propagates traceparent to Quarkus
log = logging.getLogger("checkout"); log.setLevel(logging.INFO)
tracer = trace.get_tracer("ebpf-capstone")
reqs = metrics.get_meter("ebpf-capstone").create_counter("app_requests_total")

app = FastAPI(); FastAPIInstrumentor.instrument_app(app)

@app.get("/checkout")
def checkout():
    with tracer.start_as_current_span("checkout") as span:
        tid = format(span.get_span_context().trace_id, "032x")
        stock = requests.get(INVENTORY, timeout=5).json()      # child span in Quarkus
        reqs.add(1, {"endpoint": "/checkout"})
        log.info("checkout complete stock=%s", stock)
        return {"ok": True, "trace_id": tid, "inventory": stock}
