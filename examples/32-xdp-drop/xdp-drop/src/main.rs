//! xdp-drop — attaches an XDP filter to an interface (argv[1] or $IFACE,
//! default eth0), counts IPv4 packets per protocol, and drops ICMP. Tries
//! native (driver) XDP, falls back to generic SKB mode. Exports
//! ebpf_xdp_packets_total{proto} and ebpf_xdp_dropped_total{proto}.
use std::collections::HashMap as Std;
use std::time::Duration;

use aya::{
    maps::{HashMap, MapData},
    programs::{Xdp, XdpFlags},
    Ebpf,
};
use log::{info, warn};
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use xdp_drop_common::proto_name;

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
            KeyValue::new("service.name", "ebpf-xdp-drop"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn report(
    map: &HashMap<MapData, u32, u64>,
    last: &mut Std<u32, u64>,
    counter: &Counter<u64>,
) -> anyhow::Result<()> {
    for res in map.iter() {
        let (proto, total) = res?;
        let prev = last.get(&proto).copied().unwrap_or(0);
        if total > prev {
            counter.add(total - prev, &[KeyValue::new("proto", proto_name(proto))]);
            last.insert(proto, total);
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let iface = std::env::args().nth(1)
        .or_else(|| std::env::var("IFACE").ok())
        .unwrap_or_else(|| "eth0".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/xdp-drop")))?;

    let prog: &mut Xdp = ebpf.program_mut("xdp_filter").unwrap().try_into()?;
    prog.load()?;
    // Native (driver) XDP first; fall back to generic SKB mode (e.g. in a VM).
    match prog.attach(&iface, XdpFlags::default()) {
        Ok(_) => info!("XDP attached to {iface} (native)"),
        Err(_) => {
            prog.attach(&iface, XdpFlags::SKB_MODE)?;
            warn!("XDP attached to {iface} (generic SKB_MODE — native unavailable)");
        }
    }

    let provider = init_otel()?;
    let meter = global::meter("ebpf-xdp-drop");
    let packets_total = meter.u64_counter("ebpf_xdp_packets_total").build();
    let dropped_total = meter.u64_counter("ebpf_xdp_dropped_total").build();

    let pkts: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("PKTS").unwrap())?;
    let drops: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("DROPS").unwrap())?;
    let (mut lp, mut ld) = (Std::new(), Std::new());

    println!("{:<8} {:>14} {:>10}", "PROTO", "packets", "dropped");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                report(&pkts, &mut lp, &packets_total)?;
                report(&drops, &mut ld, &dropped_total)?;
                for res in pkts.iter() {
                    let (proto, p) = res?;
                    let d = drops.get(&proto, 0).unwrap_or(0);
                    println!("{:<8} {:>14} {:>10}", proto_name(proto), p, d);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
