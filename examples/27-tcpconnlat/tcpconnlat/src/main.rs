//! tcpconnlat — reports active TCP connection latency (connect → SYN-ACK).
//! Attaches kprobes to tcp_v4_connect and tcp_rcv_state_process. Exports
//! tcp_connect_latency_ms (histogram) + ebpf_events_total{program="tcpconnlat"}.
use std::net::Ipv4Addr;
use std::time::Duration;

use aya::{maps::RingBuf, programs::KProbe, Ebpf};
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use tcpconnlat_common::{ConnEvent, COMM_LEN};

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
            KeyValue::new("service.name", "ebpf-tcpconnlat"),
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
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/tcpconnlat")))?;

    let c: &mut KProbe = ebpf.program_mut("tcp_v4_connect").unwrap().try_into()?;
    c.load()?;
    c.attach("tcp_v4_connect", 0)?;
    let r: &mut KProbe = ebpf.program_mut("tcp_rcv_state_process").unwrap().try_into()?;
    r.load()?;
    r.attach("tcp_rcv_state_process", 0)?;
    info!("tcpconnlat attached (IPv4 active connects)");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-tcpconnlat");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let hist = meter.f64_histogram("tcp_connect_latency_ms").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<16} {:<22} {}", "PID", "COMM", "DEST", "LAT(ms)");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<ConnEvent>() { continue; }
                    let ev: ConnEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const ConnEvent) };
                    let ip = Ipv4Addr::from(u32::from_be(ev.daddr));
                    let port = u16::from_be(ev.dport);
                    let ms = ev.lat_ns as f64 / 1_000_000.0;
                    println!("{:<8} {:<16} {:<22} {:.3}", ev.pid, cstr(&ev.comm[..COMM_LEN]), format!("{ip}:{port}"), ms);
                    counter.add(1, &[KeyValue::new("program", "tcpconnlat")]);
                    hist.record(ms, &[KeyValue::new("dport", port.to_string())]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
