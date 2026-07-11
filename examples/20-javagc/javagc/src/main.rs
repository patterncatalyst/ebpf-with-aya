//! javagc — time JVM GC pauses via the HotSpot USDT gc__begin/gc__end probes.
//!
//! A USDT probe is a uprobe at a fixed offset in libjvm.so. This tool takes the
//! libjvm path and the two probe offsets (resolved by the demo from the ELF
//! .note.stapsdt section) and attaches a uprobe at each:
//!   javagc LIBJVM BEGIN_OFFSET END_OFFSET
//! Offsets are decimal byte offsets into the file. Exports
//! ebpf_events_total{program="javagc"} and a GC-pause histogram.
use std::time::Duration;

use aya::{maps::RingBuf, programs::{UProbe, uprobe::UProbeScope}, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use javagc_common::{GcEvent, COMM_LEN};

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
            KeyValue::new("service.name", "ebpf-javagc"),
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
    let mut args = std::env::args().skip(1);
    let libjvm = args.next().unwrap_or_else(|| "/usr/lib/jvm/java-25/lib/server/libjvm.so".to_string());
    let begin_off: u64 = args.next().and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("usage: javagc LIBJVM BEGIN_OFFSET END_OFFSET (offsets from the demo's USDT resolver)"))?;
    let end_off: u64 = args.next().and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("missing END_OFFSET"))?;

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/javagc")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    // Attach a uprobe at each USDT probe offset (fn name = None -> use offset).
    let b: &mut UProbe = ebpf.program_mut("gc_begin").unwrap().try_into()?;
    b.load()?;
    b.attach(begin_off, &libjvm, UProbeScope::AllProcesses)?;
    let e: &mut UProbe = ebpf.program_mut("gc_end").unwrap().try_into()?;
    e.load()?;
    e.attach(end_off, &libjvm, UProbeScope::AllProcesses)?;
    info!("javagc attached to gc__begin@{begin_off} / gc__end@{end_off} in {libjvm}");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-javagc");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let hist = meter.f64_histogram("jvm_gc_pause_ms").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<16} {}", "PID", "COMM", "GC PAUSE");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<GcEvent>() { continue; }
                    let ev: GcEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const GcEvent) };
                    let ms = ev.pause_ns as f64 / 1_000_000.0;
                    println!("{:<8} {:<16} {:.3} ms", ev.pid, cstr(&ev.comm[..COMM_LEN]), ms);
                    counter.add(1, &[KeyValue::new("program", "javagc")]);
                    hist.record(ms, &[KeyValue::new("event", "gc_pause")]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
