//! he-observer — attaches uprobe/uretprobe pairs to the he_* boundaries of an HE
//! workload (argv[1], default /home/fedora/he-workload) and records a per-operation
//! latency histogram. Exports ebpf_he_op_latency_seconds{op=...} over OTLP.
//!
//! It never reads an operand. It learns four things per call: the operation name
//! (from which symbol returned), and the entry/return timestamps. That is enough
//! to tell you where an FHE workload spends its time, and nothing about the data.
use std::time::Duration;

use aya::{maps::RingBuf, programs::UProbe, Ebpf};
use he_common::Sample;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;

const SYMBOLS: [&str; 4] = ["he_keygen", "he_encrypt", "he_compute", "he_decrypt"];
const RET_PROGRAMS: [(&str, &str); 4] = [
    ("he_keygen_ret", "he_keygen"),
    ("he_encrypt_ret", "he_encrypt"),
    ("he_compute_ret", "he_compute"),
    ("he_decrypt_ret", "he_decrypt"),
];

fn op_name(op: u32) -> &'static str {
    match op {
        0 => "keygen",
        1 => "encrypt",
        2 => "compute",
        3 => "decrypt",
        _ => "other",
    }
}

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
            KeyValue::new("service.name", "ebpf-he-observer"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let target = std::env::args().nth(1).unwrap_or_else(|| "/home/fedora/he-workload".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/he-observer")))?;

    // One uprobe on entry, attached to every he_* boundary (stamps start time).
    let entry: &mut UProbe = ebpf.program_mut("he_enter").unwrap().try_into()?;
    entry.load()?;
    for sym in SYMBOLS {
        entry.attach(Some(sym), 0, &target, None)?;
    }
    // One uretprobe per boundary (records the delta, tagged with the op id).
    for (prog, sym) in RET_PROGRAMS {
        let p: &mut UProbe = ebpf.program_mut(prog).unwrap().try_into()?;
        p.load()?;
        p.attach(Some(sym), 0, &target, None)?;
    }
    info!("uprobe/uretprobe pairs attached to he_* in {target}");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-he-observer");
    let hist = meter
        .f64_histogram("ebpf_he_op_latency_seconds")
        .with_unit("s")
        .with_description("homomorphic operation latency, by op")
        .build();

    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;
    println!("{:<10} {:>14}", "OP", "DURATION(ms)");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<Sample>() { continue; }
                    let s: Sample = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const Sample) };
                    let secs = s.dur_ns as f64 / 1e9;
                    println!("{:<10} {:>14.3}", op_name(s.op), secs * 1e3);
                    hist.record(secs, &[KeyValue::new("op", op_name(s.op))]);
                }
            }
        }
    }
    if let Err(e) = provider.shutdown() { warn!("otel shutdown: {e}"); }
    Ok(())
}
