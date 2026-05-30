//! signal-kill — attach the kill-on-exec tracepoint, drain kill records, and
//! report each killed process. Exports ebpf_signal_kills_total. LAB-ONLY.
use std::time::Duration;

use aya::{maps::RingBuf, programs::TracePoint, Ebpf};
use log::info;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use signal_kill_common::KillEvent;

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
            KeyValue::new("service.name", "ebpf-signal-kill"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/signal-kill")))?;
    let prog: &mut TracePoint = ebpf.program_mut("kill_on_exec").unwrap().try_into()?;
    prog.load()?;
    prog.attach("syscalls", "sys_enter_execve")?;
    info!("watching execve — processes from /tmp/forbidden* will be killed (LAB-ONLY)");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-signal-kill");
    let kills = meter.u64_counter("ebpf_signal_kills_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("KILLS").unwrap())?;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<KillEvent>() { continue; }
                    let ev: KillEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const KillEvent) };
                    println!("killed {} (pid {})", cstr(&ev.comm), ev.pid);
                    kills.add(1, &[KeyValue::new("comm", cstr(&ev.comm))]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
