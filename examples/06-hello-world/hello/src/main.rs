//! hello — user-space loader + OTLP reporter for hello-world.
//!
//! 1. loads the embedded BPF object
//! 2. initializes aya-log so kernel info!() lines surface on the console
//! 3. attaches the `hello` tracepoint program to syscalls:sys_enter_execve
//! 4. every second, sums the per-CPU EVENTS counter and exports it to the
//!    observability stack as the OTLP metric `ebpf_events_total`
//!
//! Run on the target VM (needs CAP_BPF/CAP_SYS_ADMIN -> run under sudo).
//! Point it at the stack with OTEL_EXPORTER_OTLP_ENDPOINT (default below).

use std::time::Duration;

use aya::{maps::PerCpuArray, programs::TracePoint, Ebpf};
use aya_log::EbpfLogger;
use hello_common::EVENTS_INDEX;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;

fn init_otel() -> anyhow::Result<opentelemetry_sdk::metrics::SdkMeterProvider> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4318".to_string());
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/metrics"))
        .build()?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(
        exporter,
        opentelemetry_sdk::runtime::Tokio,
    )
    .with_interval(Duration::from_secs(2))
    .build();
    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            KeyValue::new("service.name", "ebpf-hello"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ]))
        .build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Embed + load the BPF object built by build.rs.
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/hello"
    )))?;

    if let Err(e) = EbpfLogger::init(&mut ebpf) {
        warn!("failed to initialize aya-log: {e}");
    }

    // Attach the tracepoint.
    let program: &mut TracePoint = ebpf.program_mut("hello").unwrap().try_into()?;
    program.load()?;
    program.attach("syscalls", "sys_enter_execve")?;
    info!("attached to syscalls:sys_enter_execve; reporting ebpf_events_total");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-hello");
    // A synchronous monotonic counter we add the per-second delta to. This
    // matches the Python client's `ebpf_events_total` so they share a series.
    let counter = meter.u64_counter("ebpf_events_total").build();

    // Read the per-CPU counter every second; publish how much it grew.
    let events: PerCpuArray<_, u64> = PerCpuArray::try_from(ebpf.map_mut("EVENTS").unwrap())?;
    let mut last_total: u64 = 0;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                let per_cpu = events.get(&EVENTS_INDEX, 0)?;
                let total: u64 = per_cpu.iter().copied().sum();
                let delta = total.saturating_sub(last_total);
                if delta > 0 {
                    info!("execve events observed: {total} (+{delta})");
                    counter.add(delta, &[KeyValue::new("program", "hello")]);
                    last_total = total;
                }
            }
        }
    }

    provider.shutdown()?;
    Ok(())
}
