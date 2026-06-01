//! capstone — the eBPF fourth view: per-command socket-read counts for the app
//! processes, exported as ebpf_capstone_syscalls_total{comm} to sit beside the
//! app spans/metrics/logs for the same request window.
use std::collections::HashMap as Map;
use std::time::Duration;

use aya::{maps::HashMap as BpfHashMap, programs::TracePoint, Ebpf};
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use capstone_common::Comm;

fn comm_str(c: &Comm) -> String {
    let end = c.name.iter().position(|&b| b == 0).unwrap_or(c.name.len());
    String::from_utf8_lossy(&c.name[..end]).into_owned()
}
fn init_otel() -> anyhow::Result<opentelemetry_sdk::metrics::SdkMeterProvider> {
    let ep = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:4318".into());
    let exporter = opentelemetry_otlp::MetricExporter::builder().with_http()
        .with_endpoint(format!("{ep}/v1/metrics")).build()?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_interval(Duration::from_secs(2)).build();
    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder().with_reader(reader)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            KeyValue::new("service.name", "ebpf-capstone"),
            KeyValue::new("service.namespace", "ebpf-with-aya")])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/capstone")))?;
    let prog: &mut TracePoint = ebpf.program_mut("on_read").unwrap().try_into()?;
    prog.load()?;
    prog.attach("syscalls", "sys_enter_read")?;
    println!("eBPF observer attached — counting socket reads per command");

    let provider = init_otel()?;
    let syscalls: Counter<u64> = global::meter("ebpf-capstone").u64_counter("ebpf_capstone_syscalls_total").build();
    let mut prev: Map<String, u64> = Map::new();
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let m: BpfHashMap<_, Comm, u64> = BpfHashMap::try_from(ebpf.map("SYSCALLS").unwrap())?;
        for item in m.iter() {
            let (k, v) = item?;
            let c = comm_str(&k);
            if !(c.contains("python") || c.contains("java") || c.contains("uvicorn")) { continue; }
            let d = v.saturating_sub(*prev.get(&c).unwrap_or(&0));
            if d > 0 { syscalls.add(d, &[KeyValue::new("comm", c.clone())]); }
            prev.insert(c, v);
        }
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    provider.shutdown()?;
    Ok(())
}
