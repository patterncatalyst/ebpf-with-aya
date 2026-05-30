//! lsm-fileprotect — make a file read-only (even for root) with BPF LSM.
//! Takes a path (argv[1] or $PROTECT_FILE, default /tmp/ebpf-protected),
//! stats it for its inode, and denies MAY_WRITE on it. Exports
//! ebpf_lsm_denied_total.
use std::os::unix::fs::MetadataExt;
use std::time::Duration;

use aya::{maps::{Array, HashMap}, programs::Lsm, Btf, Ebpf};
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
            KeyValue::new("service.name", "ebpf-lsm-fileprotect"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let path = std::env::args().nth(1)
        .or_else(|| std::env::var("PROTECT_FILE").ok())
        .unwrap_or_else(|| "/tmp/ebpf-protected".to_string());
    let ino = std::fs::metadata(&path)?.ino();

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/lsm-fileprotect")))?;
    {
        let mut p: Array<_, u64> = Array::try_from(ebpf.map_mut("PROTECTED").unwrap())?;
        p.set(0, ino, 0)?;
    }

    let btf = Btf::from_sys_fs()?;
    let prog: &mut Lsm = ebpf.program_mut("protect_file").unwrap().try_into()?;
    prog.load("inode_permission", &btf)?;
    prog.attach()?;
    info!("protecting {path} (inode {ino}) — writes denied, even for root");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-lsm-fileprotect");
    let denied_total = meter.u64_counter("ebpf_lsm_denied_total").build();
    let denied: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("DENIED").unwrap())?;
    let mut last = 0u64;

    println!("watching — denied write attempts to {path}:");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching — file writable again"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                let total = denied.get(&0, 0).unwrap_or(0);
                if total > last { denied_total.add(total - last, &[]); println!("denied so far: {total}"); last = total; }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
