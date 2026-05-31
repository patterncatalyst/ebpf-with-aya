//! pg-probe — attach uprobes to postgres (exec_simple_query, ProcSleep) and
//! report per-query latency (with SQL) and lock-wait time. argv[1] = postgres
//! binary path (e.g. /proc/<backend-pid>/root/usr/lib/postgresql/17/bin/postgres).
//! Exports ebpf_pg_query_duration_ms and ebpf_pg_lock_wait_ms.
use std::time::Duration;

use aya::{maps::RingBuf, programs::UProbe, Ebpf};
use log::info;
use opentelemetry::{global, metrics::Histogram, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use pg_probe_common::{Event, KIND_LOCK, KIND_QUERY};
use tokio::io::unix::AsyncFd;

fn init_otel() -> anyhow::Result<opentelemetry_sdk::metrics::SdkMeterProvider> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4318".to_string());
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http().with_endpoint(format!("{endpoint}/v1/metrics")).build()?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(
        exporter, opentelemetry_sdk::runtime::Tokio,
    ).with_interval(Duration::from_secs(2)).build();
    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            KeyValue::new("service.name", "ebpf-pg-probe"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn sql_of(e: &Event) -> String {
    let end = e.query.iter().position(|&b| b == 0).unwrap_or(e.query.len());
    String::from_utf8_lossy(&e.query[..end]).replace('\n', " ")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let target = std::env::args().nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: pg-probe <postgres-binary-path>"))?;

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/pg-probe")))?;
    for (prog, func) in [
        ("q_start", "exec_simple_query"),
        ("q_done", "exec_simple_query"),
        ("l_start", "ProcSleep"),
        ("l_done", "ProcSleep"),
    ] {
        let p: &mut UProbe = ebpf.program_mut(prog).unwrap().try_into()?;
        p.load()?;
        p.attach(Some(func), 0, &target, None)?; // pid=None: cover every backend
    }
    info!("attached uprobes to {target} (all backends)");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-pg-probe");
    let qdur: Histogram<f64> = meter.f64_histogram("ebpf_pg_query_duration_ms").build();
    let lwait: Histogram<f64> = meter.f64_histogram("ebpf_pg_lock_wait_ms").build();

    let ring = RingBuf::try_from(ebpf.take_map("EVENTS").unwrap())?;
    let mut afd = AsyncFd::new(ring)?;
    println!("{:<6} {:>10}  {}", "pid", "ms", "event");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            g = afd.readable_mut() => {
                let mut guard = g?;
                let ring = guard.get_inner_mut();
                while let Some(item) = ring.next() {
                    let e: &Event = unsafe { &*(item.as_ptr() as *const Event) };
                    let ms = e.dur_ns as f64 / 1.0e6;
                    if e.kind == KIND_QUERY {
                        qdur.record(ms, &[]);
                        println!("{:<6} {:>10.2}  query: {}", e.pid, ms, sql_of(e));
                    } else if e.kind == KIND_LOCK {
                        lwait.record(ms, &[]);
                        println!("{:<6} {:>10.2}  LOCK WAIT", e.pid, ms);
                    }
                }
                guard.clear_ready();
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
