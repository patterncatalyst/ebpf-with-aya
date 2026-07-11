//! xdp-capture — attaches an XDP capture program to an interface (argv[1] or
//! $IFACE, default eth0), prints a tcpdump-style line for each TCP SYN/FIN/RST
//! and exports ebpf_xdp_captured_total{flag} plus ebpf_xdp_seen_total{proto}.
use std::collections::HashMap as Std;
use std::net::Ipv4Addr;
use std::time::Duration;

use aya::{
    maps::{HashMap, MapData, RingBuf},
    programs::{Xdp, XdpMode},
    Ebpf,
};
use log::{info, warn};
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use xdp_capture_common::{proto_name, FlowRecord, TCP_ACK, TCP_FIN, TCP_RST, TCP_SYN};

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
            KeyValue::new("service.name", "ebpf-xdp-capture"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn flag_label(f: u8) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if f & TCP_SYN != 0 { parts.push("SYN"); }
    if f & TCP_FIN != 0 { parts.push("FIN"); }
    if f & TCP_RST != 0 { parts.push("RST"); }
    if f & TCP_ACK != 0 { parts.push("ACK"); }
    if parts.is_empty() { "·".to_string() } else { parts.join(" ") }
}

fn primary_flag(f: u8) -> &'static str {
    if f & TCP_SYN != 0 { "syn" }
    else if f & TCP_RST != 0 { "rst" }
    else if f & TCP_FIN != 0 { "fin" }
    else { "other" }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let iface = std::env::args().nth(1)
        .or_else(|| std::env::var("IFACE").ok())
        .unwrap_or_else(|| "eth0".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/xdp-capture")))?;
    let prog: &mut Xdp = ebpf.program_mut("xdp_capture").unwrap().try_into()?;
    prog.load()?;
    match prog.attach(&iface, XdpMode::default()) {
        Ok(_) => info!("XDP capture attached to {iface} (native)"),
        Err(_) => { prog.attach(&iface, XdpMode::Skb)?; warn!("XDP attached to {iface} (SKB_MODE)"); }
    }

    let provider = init_otel()?;
    let meter = global::meter("ebpf-xdp-capture");
    let captured: Counter<u64> = meter.u64_counter("ebpf_xdp_captured_total").build();
    let seen_total: Counter<u64> = meter.u64_counter("ebpf_xdp_seen_total").build();

    let mut ring = RingBuf::try_from(ebpf.take_map("EVENTS").unwrap())?;
    let seen: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("SEEN").unwrap())?;
    let mut last: Std<u32, u64> = Std::new();

    println!("{:<10} {:<22} {:<22} {}", "FLAGS", "SRC", "DST", "LEN");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<FlowRecord>() { continue; }
                    let r: FlowRecord = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const FlowRecord) };
                    let src = format!("{}:{}", Ipv4Addr::from(u32::from_be(r.saddr)), r.sport);
                    let dst = format!("{}:{}", Ipv4Addr::from(u32::from_be(r.daddr)), r.dport);
                    println!("{:<10} {:<22} {:<22} {}", flag_label(r.flags), src, dst, r.len);
                    captured.add(1, &[KeyValue::new("flag", primary_flag(r.flags))]);
                }
                for res in seen.iter() {
                    let (proto, total) = res?;
                    let prev = last.get(&proto).copied().unwrap_or(0);
                    if total > prev { seen_total.add(total - prev, &[KeyValue::new("proto", proto_name(proto))]); last.insert(proto, total); }
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
