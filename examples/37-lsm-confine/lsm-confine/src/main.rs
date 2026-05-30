//! lsm-confine — confine a cgroup's outbound connections with BPF LSM.
//! Takes a cgroup-v2 path (argv[1] or $CONFINE_CGROUP, default
//! /sys/fs/cgroup/confined), stats it for its id, and denies connect() for
//! processes in it. Exports ebpf_lsm_denied_total.
use std::os::unix::fs::MetadataExt;
use std::time::Duration;

use aya::{maps::HashMap, programs::Lsm, Btf, Ebpf};
use log::info;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;

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
            KeyValue::new("service.name", "ebpf-lsm-confine"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cg_path = std::env::args().nth(1)
        .or_else(|| std::env::var("CONFINE_CGROUP").ok())
        .unwrap_or_else(|| "/sys/fs/cgroup/confined".to_string());
    let cgid = std::fs::metadata(&cg_path)?.ino();

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/lsm-confine")))?;

    {
        let mut confined: HashMap<_, u64, u8> = HashMap::try_from(ebpf.map_mut("CONFINED").unwrap())?;
        confined.insert(cgid, 1, 0)?;
    }

    let btf = Btf::from_sys_fs()?;
    let prog: &mut Lsm = ebpf.program_mut("confine_connect").unwrap().try_into()?;
    prog.load("socket_connect", &btf)?;
    prog.attach()?;
    info!("confining cgroup id {cgid} ({cg_path}) — connect() denied for it");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-lsm-confine");
    let denied_total = meter.u64_counter("ebpf_lsm_denied_total").build();
    let denied: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("DENIED").unwrap())?;
    let mut last = 0u64;

    println!("watching — denied connect() attempts from the confined cgroup:");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                let total = denied.get(&0, 0).unwrap_or(0);
                if total > last {
                    denied_total.add(total - last, &[]);
                    println!("denied so far: {total}");
                    last = total;
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
