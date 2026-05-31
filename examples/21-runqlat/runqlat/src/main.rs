//! runqlat — reads the in-kernel log2 histogram of run-queue latency, prints an
//! ASCII histogram, and exports approximate percentiles (p50/p90/p99) as an
//! OTLP observable gauge `runqueue_latency_us{quantile=...}`.
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aya::{maps::Array, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use runqlat_common::NBUCKETS;

type Buckets = [u64; NBUCKETS as usize];

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
            KeyValue::new("service.name", "ebpf-runqlat"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

// Approximate a percentile from log2-us buckets: return the bucket's upper edge.
fn percentile_us(b: &Buckets, q: f64) -> f64 {
    let total: u64 = b.iter().sum();
    if total == 0 { return 0.0; }
    let target = (total as f64 * q) as u64;
    let mut cum = 0u64;
    for (i, &c) in b.iter().enumerate() {
        cum += c;
        if cum >= target { return (1u64 << (i + 1)) as f64; } // upper edge, µs
    }
    (1u64 << NBUCKETS) as f64
}

fn print_hist(b: &Buckets) {
    let total: u64 = b.iter().sum();
    if total == 0 { return; }
    let max = b.iter().copied().max().unwrap_or(1).max(1);
    println!("\nrun-queue latency (usec)   total={total}");
    for (i, &c) in b.iter().enumerate() {
        if c == 0 { continue; }
        let lo = 1u64 << i;
        let hi = (1u64 << (i + 1)) - 1;
        let bars = (c as f64 / max as f64 * 32.0) as usize;
        println!("{:>10} -> {:<10} {:>8} |{}", lo, hi, c, "*".repeat(bars));
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/runqlat")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    for tp in ["sched_wakeup", "sched_wakeup_new", "sched_switch"] {
        let p: &mut aya::programs::TracePoint = ebpf.program_mut(tp).unwrap().try_into()?;
        p.load()?;
        p.attach("sched", tp)?;
    }
    info!("runqlat attached to sched_wakeup/_new + sched_switch");

    // Shared snapshot the OTLP gauge callback reads.
    let snap: Arc<Mutex<Buckets>> = Arc::new(Mutex::new([0u64; NBUCKETS as usize]));

    let provider = init_otel()?;
    let meter = global::meter("ebpf-runqlat");
    {
        let snap = snap.clone();
        // Registered ONCE; the SDK calls it at each export.
        let _gauge = meter
            .f64_observable_gauge("ebpf_runqueue_latency_us")
            .with_callback(move |obs| {
                let b = *snap.lock().unwrap();
                for (q, name) in [(0.50, "p50"), (0.90, "p90"), (0.99, "p99")] {
                    obs.observe(percentile_us(&b, q), &[KeyValue::new("quantile", name)]);
                }
            })
            .build();
    }

    let hist: Array<_, u64> = Array::try_from(ebpf.map("HIST").unwrap())?;
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tick.tick() => {
                let mut b = [0u64; NBUCKETS as usize];
                for i in 0..NBUCKETS { b[i as usize] = hist.get(&i, 0).unwrap_or(0); }
                *snap.lock().unwrap() = b;
                print_hist(&b);
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
