//! core — load the CO-RE reader (relocations resolve against THIS kernel's BTF
//! at load), drive getpid, and export the count of portable reads.
use std::time::Duration;

use aya::{maps::Array, programs::KProbe, Ebpf};
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
            KeyValue::new("service.name", "ebpf-core"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    // BTF is loaded transparently; the kprobe's field reads relocate at load.
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/core")))?;
    let prog: &mut KProbe = ebpf.program_mut("count_reads").unwrap().try_into()?;
    prog.load()?;
    prog.attach("__x64_sys_getpid", 0)?;
    info!("CO-RE reader loaded; relocations resolved against this kernel's BTF");

    let provider = init_otel()?;
    let counter = global::meter("ebpf-core").u64_counter("ebpf_core_reads_total").build();

    let mut last = 0u64;
    for _ in 0..20 {
        for _ in 0..50 { unsafe { libc::getpid() }; }
        tokio::time::sleep(Duration::from_secs(1)).await;
        let reads: Array<_, u64> = Array::try_from(ebpf.map("READS").unwrap())?;
        let now = reads.get(&0, 0).unwrap_or(0);
        counter.add(now.saturating_sub(last), &[]);
        println!("portable reads: {now}");
        last = now;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    provider.shutdown()?;
    Ok(())
}
