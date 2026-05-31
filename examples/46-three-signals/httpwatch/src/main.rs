//! httpwatch — turn one socket-observed request into three correlated OTel
//! signals: a span (Tempo), a log (Loki), and RED metrics (Prometheus), all
//! sharing one trace_id. A teaching-grade sketch of what OBI does for real.
//!
//! UNVERIFIED: the opentelemetry 0.27 traces/logs builder surface used here
//! (tracer/span timing, manual trace_id, log bridge) and exemplar emission
//! must be confirmed against a real build; see the chapter's status note.
use std::time::{Duration, SystemTime};

use aya::{
    maps::{HashMap as AyaMap, MapData, RingBuf},
    programs::KProbe,
    Ebpf,
};
use httpwatch_common::Req;
use log::info;
use opentelemetry::{
    global,
    metrics::{Counter, Histogram},
    trace::{SpanBuilder, TraceContextExt, Tracer, TraceId},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tokio::io::unix::AsyncFd;

fn endpoint() -> String {
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:4318".into())
}
fn resource() -> opentelemetry_sdk::Resource {
    opentelemetry_sdk::Resource::new(vec![
        KeyValue::new("service.name", "ebpf-three-signals"),
        KeyValue::new("service.namespace", "ebpf-with-aya"),
    ])
}

fn svc_from(comm: &[u8; 16]) -> String {
    let s = String::from_utf8_lossy(comm);
    let s = s.trim_end_matches('\0');
    if s.starts_with("java") { "java".into() }
    else if s.starts_with("python") || s.contains("uvicorn") { "python".into() }
    else { s.to_string() }
}

fn init_metrics() -> anyhow::Result<opentelemetry_sdk::metrics::SdkMeterProvider> {
    let exp = opentelemetry_otlp::MetricExporter::builder()
        .with_http().with_endpoint(format!("{}/v1/metrics", endpoint())).build()?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exp, opentelemetry_sdk::runtime::Tokio)
        .with_interval(Duration::from_secs(2)).build();
    let p = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader).with_resource(resource()).build();
    global::set_meter_provider(p.clone());
    Ok(p)
}
fn init_traces() -> anyhow::Result<opentelemetry_sdk::trace::TracerProvider> {
    let exp = opentelemetry_otlp::SpanExporter::builder()
        .with_http().with_endpoint(format!("{}/v1/traces", endpoint())).build()?;
    let p = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exp, opentelemetry_sdk::runtime::Tokio)
        .with_resource(resource()).build();
    global::set_tracer_provider(p.clone());
    Ok(p)
}
fn init_logs() -> anyhow::Result<opentelemetry_sdk::logs::LoggerProvider> {
    let exp = opentelemetry_otlp::LogExporter::builder()
        .with_http().with_endpoint(format!("{}/v1/logs", endpoint())).build()?;
    Ok(opentelemetry_sdk::logs::LoggerProvider::builder()
        .with_batch_exporter(exp, opentelemetry_sdk::runtime::Tokio)
        .with_resource(resource()).build())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/httpwatch")))?;
    for prog in ["on_recv", "on_send"] {
        let p: &mut KProbe = ebpf.program_mut(prog).unwrap().try_into()?;
        p.load()?;
    }
    // attach: tcp_recvmsg stamps the request start, tcp_sendmsg closes it
    let recv: &mut KProbe = ebpf.program_mut("on_recv").unwrap().try_into()?;
    recv.attach("tcp_recvmsg", 0)?;
    let send: &mut KProbe = ebpf.program_mut("on_send").unwrap().try_into()?;
    send.attach("tcp_sendmsg", 0)?;
    info!("socket probe attached (tcp_recvmsg / tcp_sendmsg)");

    let metrics = init_metrics()?;
    let traces = init_traces()?;
    let logs = init_logs()?;
    let meter = global::meter("ebpf-three-signals");
    let requests: Counter<u64> = meter.u64_counter("ebpf_http_server_requests_total").build();
    let duration: Histogram<f64> = meter.f64_histogram("ebpf_http_server_duration_ms").build();
    let tracer = global::tracer("ebpf-three-signals");

    let ring = RingBuf::try_from(ebpf.take_map("EVENTS").unwrap())?;
    let mut afd = AsyncFd::new(ring)?;
    println!("{:<12} {:>12}", "service", "duration");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            g = afd.readable_mut() => {
                let mut guard = g?;
                let ring = guard.get_inner_mut();
                while let Some(item) = ring.next() {
                    let e: &Req = unsafe { &*(item.as_ptr() as *const Req) };
                    let svc = svc_from(&e.comm);
                    let ms = e.dur_ns as f64 / 1.0e6;
                    let end = SystemTime::now();
                    let start = end - Duration::from_nanos(e.dur_ns);

                    // one trace_id across all three signals
                    let trace_id = TraceId::from_bytes(rand_bytes16());
                    let span = tracer.build(SpanBuilder::from_name("http.server.request")
                        .with_trace_id(trace_id)
                        .with_start_time(start)
                        .with_end_time(end)
                        .with_attributes(vec![KeyValue::new("service.name", svc.clone())]));
                    let cx = opentelemetry::Context::current_with_span(span);

                    // log -> Loki, carrying the trace context for correlation
                    emit_log(&logs, &svc, ms, trace_id);
                    // RED metrics -> Prometheus (exemplar carries trace_id where supported)
                    let _ = &cx; // span ends on drop
                    requests.add(1, &[KeyValue::new("service", svc.clone())]);
                    duration.record(ms, &[KeyValue::new("service", svc.clone())]);

                    println!("{:<12} {:>9.0} µs", svc, e.dur_ns as f64 / 1000.0);
                }
                guard.clear_ready();
            }
        }
    }
    let _ = traces.shutdown(); let _ = logs.shutdown(); let _ = metrics.shutdown();
    Ok(())
}

fn rand_bytes16() -> [u8; 16] {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let mut b = [0u8; 16];
    b[..16].copy_from_slice(&n.to_le_bytes()[..16]);
    // mix in the address of a stack local for a little more entropy
    let salt = (&b as *const _ as u64).to_le_bytes();
    for (i, s) in salt.iter().enumerate() { b[i] ^= s; }
    b
}

fn emit_log(
    logs: &opentelemetry_sdk::logs::LoggerProvider,
    svc: &str, ms: f64, trace_id: TraceId,
) {
    use opentelemetry::logs::{LogRecord, Logger, LoggerProvider, Severity};
    let logger = logs.logger("ebpf-three-signals");
    let mut rec = logger.create_log_record();
    rec.set_severity_number(Severity::Info);
    rec.set_body(format!("{svc} handled request in {:.1} ms", ms).into());
    rec.add_attribute("service", svc.to_string());
    rec.add_attribute("trace_id", format!("{:032x}", u128::from_be_bytes(trace_id.to_bytes())));
    logger.emit(rec);
}
