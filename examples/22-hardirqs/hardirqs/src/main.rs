//! hardirqs — reads the in-kernel per-IRQ totals, prints a table, and exports
//! per-IRQ total handler time as an OTLP observable gauge
//! `hardirq_total_ns{irq=...}`, plus a counter of IRQs handled.
use std::collections::HashMap as Std;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aya::maps::HashMap as BpfHashMap;
use aya::Ebpf;
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use hardirqs_common::IrqStat;

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
            KeyValue::new("service.name", "ebpf-hardirqs"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/hardirqs")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    for tp in ["irq_handler_entry", "irq_handler_exit"] {
        let p: &mut aya::programs::TracePoint = ebpf.program_mut(tp).unwrap().try_into()?;
        p.load()?;
        p.attach("irq", tp)?;
    }
    info!("hardirqs attached to irq_handler_entry/exit");

    // Snapshot of irq -> total_ns for the OTLP gauge callback.
    let snap: Arc<Mutex<Std<u32, u64>>> = Arc::new(Mutex::new(Std::new()));

    let provider = init_otel()?;
    let meter = global::meter("ebpf-hardirqs");
    let counter = meter.u64_counter("ebpf_events_total").build();
    {
        let snap = snap.clone();
        let _gauge = meter.u64_observable_gauge("hardirq_total_ns")
            .with_callback(move |obs| {
                for (irq, ns) in snap.lock().unwrap().iter() {
                    obs.observe(*ns, &[KeyValue::new("program", "hardirqs"), KeyValue::new("irq", irq.to_string())]);
                }
            }).build();
    }

    let mut last_total: u64 = 0;
    let mut tick = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tick.tick() => {
                let hist: BpfHashMap<_, u32, IrqStat> = BpfHashMap::try_from(ebpf.map("HIST").unwrap())?;
                let mut rows: Vec<(u32, IrqStat)> = Vec::new();
                for item in hist.iter() {
                    if let Ok((irq, st)) = item { rows.push((irq, st)); }
                }
                rows.sort_by(|a, b| b.1.total_ns.cmp(&a.1.total_ns));
                let mut snapshot = Std::new();
                let mut total_count = 0u64;
                println!("\n{:<8} {:>10} {:>14} {:>12}", "IRQ", "COUNT", "TOTAL(us)", "AVG(ns)");
                for (irq, st) in &rows {
                    let avg = if st.count > 0 { st.total_ns / st.count } else { 0 };
                    println!("{:<8} {:>10} {:>14} {:>12}", irq, st.count, st.total_ns / 1000, avg);
                    snapshot.insert(*irq, st.total_ns);
                    total_count += st.count;
                }
                *snap.lock().unwrap() = snapshot;
                // export the delta in total IRQs handled since last tick
                if total_count >= last_total { counter.add(total_count - last_total, &[KeyValue::new("program", "hardirqs")]); }
                last_total = total_count;
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
