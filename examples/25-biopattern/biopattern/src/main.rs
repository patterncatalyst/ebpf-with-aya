//! biopattern — reads the per-device sequential/random counters, prints a table
//! every 2s, and exports the sequential ratio as an OTLP observable gauge
//! `bio_sequential_ratio{dev}`. Device shown as major:minor.
use std::collections::HashMap as Std;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aya::maps::HashMap as BpfHashMap;
use aya::Ebpf;
use log::info;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use biopattern_common::BioStat;

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
            KeyValue::new("service.name", "ebpf-biopattern"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

// dev_t -> "major:minor" (Linux encoding).
fn devname(dev: u32) -> String {
    let major = (dev >> 20) & 0xfff;
    let minor = dev & 0xfffff;
    format!("{major}:{minor}")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/biopattern")))?;

    let tp: &mut aya::programs::TracePoint = ebpf.program_mut("block_rq_issue").unwrap().try_into()?;
    tp.load()?;
    tp.attach("block", "block_rq_issue")?;
    info!("biopattern attached to block:block_rq_issue");

    // dev -> sequential ratio (0..1) for the OTLP gauge callback.
    let snap: Arc<Mutex<Std<u32, f64>>> = Arc::new(Mutex::new(Std::new()));
    let provider = init_otel()?;
    let meter = global::meter("ebpf-biopattern");
    {
        let snap = snap.clone();
        let _g = meter.f64_observable_gauge("bio_sequential_ratio")
            .with_callback(move |obs| {
                for (dev, ratio) in snap.lock().unwrap().iter() {
                    obs.observe(*ratio, &[KeyValue::new("dev", devname(*dev))]);
                }
            }).build();
    }

    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tick.tick() => {
                let stats: BpfHashMap<_, u32, BioStat> = BpfHashMap::try_from(ebpf.map("STATS").unwrap())?;
                let mut ratios = Std::new();
                println!("\n{:<10} {:>10} {:>10} {:>8} {:>12}", "DEV", "SEQ", "RANDOM", "SEQ%", "TOTAL(MB)");
                for item in stats.iter() {
                    if let Ok((dev, s)) = item {
                        let total = s.sequential + s.random;
                        let pct = if total > 0 { s.sequential as f64 / total as f64 } else { 0.0 };
                        ratios.insert(dev, pct);
                        println!("{:<10} {:>10} {:>10} {:>7.0}% {:>12.1}",
                            devname(dev), s.sequential, s.random, pct * 100.0, s.bytes as f64 / 1_048_576.0);
                    }
                }
                *snap.lock().unwrap() = ratios;
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
