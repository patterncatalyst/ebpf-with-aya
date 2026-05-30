//! sockops — attaches the sock_ops program to a cgroup (argv[1], default the
//! cgroup-v2 root /sys/fs/cgroup) and prints established TCP connections by
//! direction. Exports ebpf_sock_established_total{dir}.
use std::net::Ipv4Addr;
use std::time::Duration;

use aya::{maps::RingBuf, programs::SockOps, Ebpf};
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use sockops_common::{SockEvent, DIR_ACTIVE};

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
            KeyValue::new("service.name", "ebpf-sockops"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cgroup_path = std::env::args().nth(1).unwrap_or_else(|| "/sys/fs/cgroup".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/sockops")))?;
    let cgroup = std::fs::File::open(&cgroup_path)?;
    let prog: &mut SockOps = ebpf.program_mut("track").unwrap().try_into()?;
    prog.load()?;
    prog.attach(cgroup)?;
    info!("sock_ops attached to cgroup {cgroup_path}");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-sockops");
    let counter = meter.u64_counter("ebpf_sock_established_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<22} {}", "DIR", "LOCAL", "REMOTE");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<SockEvent>() { continue; }
                    let ev: SockEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const SockEvent) };
                    let dir = if ev.dir == DIR_ACTIVE { "active" } else { "passive" };
                    let local = format!("{}:{}", Ipv4Addr::from(u32::from_be(ev.local_ip4)), ev.local_port);
                    let remote = format!("{}:{}", Ipv4Addr::from(u32::from_be(ev.remote_ip4)), u16::from_be(ev.remote_port));
                    println!("{:<8} {:<22} {}", dir, local, remote);
                    counter.add(1, &[KeyValue::new("dir", dir)]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
