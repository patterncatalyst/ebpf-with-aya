//! sslsniff — attaches SSL_write/SSL_read uprobes to libssl and prints the
//! plaintext crossing TLS. Target lib via argv[1] (default
//! /usr/lib64/libssl.so.3). Exports ebpf_events_total{program="sslsniff",dir}.
use std::time::Duration;

use aya::{maps::RingBuf, programs::{UProbe, uprobe::UProbeScope}, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use sslsniff_common::{TlsEvent, COMM_LEN, DATA_CAP, DIR_READ};

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
            KeyValue::new("service.name", "ebpf-sslsniff"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

// Render captured bytes printable-ish for a one-line preview.
fn preview(data: &[u8]) -> String {
    data.iter().map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' }).collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let lib = std::env::args().nth(1).unwrap_or_else(|| "/usr/lib64/libssl.so.3".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/sslsniff")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let w: &mut UProbe = ebpf.program_mut("ssl_write").unwrap().try_into()?;
    w.load()?;
    w.attach("SSL_write", &lib, UProbeScope::AllProcesses)?;
    let re: &mut UProbe = ebpf.program_mut("ssl_read_enter").unwrap().try_into()?;
    re.load()?;
    re.attach("SSL_read", &lib, UProbeScope::AllProcesses)?;
    let rr: &mut UProbe = ebpf.program_mut("ssl_read_ret").unwrap().try_into()?;
    rr.load()?;
    rr.attach("SSL_read", &lib, UProbeScope::AllProcesses)?;
    info!("sslsniff attached to SSL_write/SSL_read in {lib}");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-sslsniff");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<16} {:<6} {:<6} {}", "PID", "COMM", "DIR", "LEN", "DATA");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<TlsEvent>() { continue; }
                    let ev: TlsEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const TlsEvent) };
                    let dir = if ev.dir == DIR_READ { "READ" } else { "WRITE" };
                    let cap = (ev.captured as usize).min(DATA_CAP);
                    println!("{:<8} {:<16} {:<6} {:<6} {}", ev.pid, cstr(&ev.comm[..COMM_LEN]), dir, ev.len, preview(&ev.data[..cap]));
                    counter.add(1, &[KeyValue::new("program", "sslsniff"), KeyValue::new("dir", dir)]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
