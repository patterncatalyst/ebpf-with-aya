//! xdp-lb — a UDP port load balancer. Fills the BACKENDS map (default ports
//! 9001-9003, override with $BACKENDS="9001,9002"), attaches XDP to the
//! interface (argv[1] or $IFACE, default eth0), and exports per-backend
//! dispatch counts as ebpf_xdp_lb_dispatch_total{backend}.
use std::collections::HashMap as Std;
use std::time::Duration;

use aya::{
    maps::{Array, HashMap, MapData},
    programs::{Xdp, XdpFlags},
    Ebpf,
};
use log::{info, warn};
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use xdp_lb_common::VIP_PORT;

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
            KeyValue::new("service.name", "ebpf-xdp-lb"),
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
    let backends: Vec<u16> = std::env::var("BACKENDS")
        .unwrap_or_else(|_| "9001,9002,9003".to_string())
        .split(',').filter_map(|s| s.trim().parse().ok()).collect();

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/xdp-lb")))?;

    // Program the backend set before attaching.
    {
        let mut b: Array<_, u16> = Array::try_from(ebpf.map_mut("BACKENDS").unwrap())?;
        for (i, p) in backends.iter().enumerate() { b.set(i as u32, *p, 0)?; }
        let mut nb: Array<_, u32> = Array::try_from(ebpf.map_mut("NBACK").unwrap())?;
        nb.set(0, backends.len() as u32, 0)?;
        let mut idx: Array<_, u32> = Array::try_from(ebpf.map_mut("IDX").unwrap())?;
        idx.set(0, 0, 0)?;
    }

    let prog: &mut Xdp = ebpf.program_mut("xdp_lb").unwrap().try_into()?;
    prog.load()?;
    match prog.attach(&iface, XdpFlags::default()) {
        Ok(_) => info!("XDP LB attached to {iface} (native); VIP_PORT={VIP_PORT} backends={backends:?}"),
        Err(_) => { prog.attach(&iface, XdpFlags::SKB_MODE)?; warn!("XDP LB attached to {iface} (SKB_MODE)"); }
    }

    let provider = init_otel()?;
    let meter = global::meter("ebpf-xdp-lb");
    let dispatch: Counter<u64> = meter.u64_counter("ebpf_xdp_lb_dispatch_total").build();
    let hits: HashMap<_, u16, u64> = HashMap::try_from(ebpf.take_map("HITS").unwrap())?;
    let mut last: Std<u16, u64> = Std::new();

    println!("{:<10} {:>14}", "BACKEND", "dispatched");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                for res in hits.iter() {
                    let (port, total) = res?;
                    let prev = last.get(&port).copied().unwrap_or(0);
                    if total > prev {
                        dispatch.add(total - prev, &[KeyValue::new("backend", port.to_string())]);
                        last.insert(port, total);
                    }
                    println!("{:<10} {:>14}", port, total);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
