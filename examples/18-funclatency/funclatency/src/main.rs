//! funclatency — time a function (uprobe entry + uretprobe exit). Records each
//! call's duration into an OTLP histogram (ms) and prints a running ASCII
//! histogram. Usage: funclatency BIN SYMBOL  (default target-app slow_op).
use std::time::Duration;

use aya::{maps::RingBuf, programs::UProbe, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use funclatency_common::LatEvent;

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
            KeyValue::new("service.name", "ebpf-funclatency"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let bin = std::env::args().nth(1).unwrap_or_else(|| "/home/fedora/target-app".to_string());
    let sym = std::env::args().nth(2).unwrap_or_else(|| "slow_op".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/funclatency")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let en: &mut UProbe = ebpf.program_mut("fn_enter").unwrap().try_into()?;
    en.load()?;
    en.attach(Some(&sym), 0, &bin, None)?;
    let ex: &mut UProbe = ebpf.program_mut("fn_exit").unwrap().try_into()?;
    ex.load()?;
    ex.attach(Some(&sym), 0, &bin, None)?;
    info!("funclatency timing {sym} in {bin}");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-funclatency");
    let hist = meter.f64_histogram("function_latency_ms").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    // log2(microseconds) buckets for a console histogram.
    let mut buckets = [0u64; 32];
    let mut count = 0u64;
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tick.tick() => { print_hist(&buckets, count, &sym); }
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<LatEvent>() { continue; }
                    let ev: LatEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const LatEvent) };
                    let ms = ev.delta_ns as f64 / 1_000_000.0;
                    hist.record(ms, &[KeyValue::new("symbol", sym.clone())]);
                    let us = (ev.delta_ns / 1000).max(1);
                    let b = (63 - us.leading_zeros()) as usize; // floor(log2(us))
                    buckets[b.min(31)] += 1;
                    count += 1;
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}

fn print_hist(buckets: &[u64; 32], count: u64, sym: &str) {
    if count == 0 { return; }
    let max = buckets.iter().copied().max().unwrap_or(1).max(1);
    println!("\n{sym}: {count} calls");
    println!("{:>10} {:>8}  {}", "usec", "count", "distribution");
    for (i, &c) in buckets.iter().enumerate() {
        if c == 0 { continue; }
        let lo = 1u64 << i;
        let hi = (1u64 << (i + 1)) - 1;
        let bars = (c as f64 / max as f64 * 32.0) as usize;
        println!("{:>4} -> {:<4} {:>8}  |{}", lo, hi, c, "*".repeat(bars));
    }
}
