//! tcx — attaches an ingress classifier to an interface (argv[1] or $IFACE,
//! default eth0) via tcx (kernel 6.6+): no clsact qdisc, a bpf_link that
//! auto-detaches when dropped. Exports ebpf_tcx_packets_total{proto}.
use std::collections::HashMap as Std;
use std::time::Duration;

use aya::{
    maps::{HashMap, MapData},
    programs::{tc::TcAttachType, SchedClassifier},
    Ebpf,
};
use log::info;
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;

fn proto_name(p: u32) -> &'static str {
    match p {
        1 => "icmp",
        6 => "tcp",
        17 => "udp",
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
            KeyValue::new("service.name", "ebpf-tcx"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let iface = std::env::args().nth(1)
        .or_else(|| std::env::var("IFACE").ok())
        .unwrap_or_else(|| "eth0".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/tcx")))?;

    // tcx: no qdisc_add_clsact. attach returns a link that owns the attachment;
    // keep it alive for the process lifetime — dropping it detaches.
    let prog: &mut SchedClassifier = ebpf.program_mut("tcx_count").unwrap().try_into()?;
    prog.load()?;
    let _link = prog.attach(&iface, TcAttachType::Ingress)?;
    info!("tcx classifier attached to {iface} ingress (no clsact qdisc)");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-tcx");
    let packets: Counter<u64> = meter.u64_counter("ebpf_tcx_packets_total").build();
    let pkts: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("PKTS").unwrap())?;
    let mut last: Std<u32, u64> = Std::new();

    println!("{:<8} {:>14}", "PROTO", "packets");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching (dropping the tcx link)"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                for res in pkts.iter() {
                    let (proto, total) = res?;
                    let prev = last.get(&proto).copied().unwrap_or(0);
                    if total > prev {
                        packets.add(total - prev, &[KeyValue::new("proto", proto_name(proto))]);
                        last.insert(proto, total);
                    }
                    println!("{:<8} {:>14}", proto_name(proto), total);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
