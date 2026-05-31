//! offload — attach the XDP counter, asking for HARDWARE offload first, then
//! native, then generic, and report which mode actually engaged. On the lab's
//! virtio NIC expect DRV or SKB (host CPU): there's no offload NIC here.
use std::time::Duration;

use aya::{maps::Array, programs::{Xdp, XdpFlags}, Ebpf};
use log::info;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;

fn init_otel() -> anyhow::Result<opentelemetry_sdk::metrics::SdkMeterProvider> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4318".to_string());
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http().with_endpoint(format!("{endpoint}/v1/metrics")).build()?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(
        exporter, opentelemetry_sdk::runtime::Tokio).with_interval(Duration::from_secs(2)).build();
    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            KeyValue::new("service.name", "ebpf-offload"),
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

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/offload")))?;
    let prog: &mut Xdp = ebpf.program_mut("count").unwrap().try_into()?;
    prog.load()?;

    // walk down the modes: HW (offload, on the NIC) → DRV (native, host CPU) → SKB (generic)
    let mut engaged = "none";
    for (name, flag) in [("HW", XdpFlags::HW_MODE), ("DRV", XdpFlags::DRV_MODE), ("SKB", XdpFlags::SKB_MODE)] {
        match prog.attach(&iface, flag) {
            Ok(_) => { engaged = name; break; }
            Err(e) => info!("{name} mode unavailable on {iface}: {e}"),
        }
    }
    println!("XDP on {iface}: engaged {engaged} mode  (HW = offloaded to the NIC; DRV/SKB = host CPU)");
    if engaged == "HW" {
        println!("  → packets are processed ON THE NIC, off the host CPU");
    } else if engaged != "none" {
        println!("  → no offload NIC here; running on the host CPU (expected on virtio)");
    }

    let provider = init_otel()?;
    let counter = global::meter("ebpf-offload").u64_counter("ebpf_offload_packets_total").build();
    let pkts: Array<_, u64> = Array::try_from(ebpf.map("PKTS").unwrap())?;
    let mut last = 0u64;
    for _ in 0..15 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let now = pkts.get(&0, 0).unwrap_or(0);
        counter.add(now.saturating_sub(last), &[]);
        println!("packets seen by XDP: {now}");
        last = now;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    provider.shutdown()?;
    Ok(())
}
