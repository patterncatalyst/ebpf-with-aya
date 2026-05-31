//! cpu-busy — attach a sched_switch probe and report per-CPU busy percent each
//! interval, exporting ebpf_cpu_busy_ns_total{cpu}. Under scx_nest at moderate
//! load a few cores run hot while the rest stay idle — the nest made visible.
use std::collections::HashMap as Std;
use std::time::{Duration, Instant};

use aya::{maps::HashMap, programs::TracePoint, Ebpf};
use log::info;
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;

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
            KeyValue::new("service.name", "ebpf-cpu-busy"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/cpu-busy")))?;
    let prog: &mut TracePoint = ebpf.program_mut("on_switch").unwrap().try_into()?;
    prog.load()?;
    prog.attach("sched", "sched_switch")?;
    info!("per-CPU busy probe attached");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-cpu-busy");
    let busy_ns: Counter<u64> = meter.u64_counter("ebpf_cpu_busy_ns_total").build();
    let map: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("BUSY").unwrap())?;
    let mut last: Std<u32, u64> = Std::new();
    let mut t0 = Instant::now();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                let wall = t0.elapsed().as_nanos() as u64;
                t0 = Instant::now();
                let mut rows: Vec<(u32, f64)> = Vec::new();
                for res in map.iter() {
                    let (cpu, total) = res?;
                    let prev = last.get(&cpu).copied().unwrap_or(0);
                    let delta = total.saturating_sub(prev);
                    last.insert(cpu, total);
                    if delta > 0 { busy_ns.add(delta, &[KeyValue::new("cpu", cpu.to_string())]); }
                    let pct = if wall > 0 { (delta as f64 / wall as f64 * 100.0).min(100.0) } else { 0.0 };
                    rows.push((cpu, pct));
                }
                rows.sort_by_key(|r| r.0);
                let bar = rows.iter().map(|(c, p)| format!("cpu{c}:{p:>3.0}%")).collect::<Vec<_>>().join("  ");
                println!("{bar}");
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
