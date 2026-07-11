//! btf-uprobe — attaches a uprobe to process_order in the target binary
//! (argv[1], default /home/fedora/target-app) and reports the Order struct it
//! read from each call. Exports ebpf_events_total{program="btf-uprobe",status}.
use std::time::Duration;

use aya::{maps::RingBuf, programs::{UProbe, uprobe::UProbeScope}, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use btf_uprobe_common::OrderEvent;

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
            KeyValue::new("service.name", "ebpf-btf-uprobe"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn status_name(s: u32) -> &'static str {
    match s { 0 => "received", 1 => "paid", 2 => "shipped", _ => "unknown" }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let target = std::env::args().nth(1).unwrap_or_else(|| "/home/fedora/target-app".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/btf-uprobe")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let prog: &mut UProbe = ebpf.program_mut("process_order_enter").unwrap().try_into()?;
    prog.load()?;
    prog.attach("process_order", &target, UProbeScope::AllProcesses)?;
    info!("uprobe attached to process_order in {target}");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-btf-uprobe");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<8} {:<12} {}", "PID", "ID", "AMOUNT", "STATUS");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<OrderEvent>() { continue; }
                    let ev: OrderEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const OrderEvent) };
                    let st = status_name(ev.order.status);
                    let amount = format!("${}.{:02}", ev.order.amount_cents / 100, ev.order.amount_cents % 100);
                    println!("{:<8} {:<8} {:<12} {}", ev.pid, ev.order.id, amount, st);
                    counter.add(1, &[KeyValue::new("program", "btf-uprobe"), KeyValue::new("status", st)]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
