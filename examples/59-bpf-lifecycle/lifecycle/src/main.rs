//! lifecycle — load the counter, PIN its map and link to bpffs so they outlive
//! this process, drive events for a few seconds, and exit leaving them pinned.
//! Run again and it REUSES the pinned map: the count continues, not resets —
//! demonstrating lifetime decoupling (pinned link) + state continuity (pinned map).
use std::path::Path;
use std::time::Duration;

use aya::{
    maps::Array,
    programs::{links::FdLink, TracePoint},
    EbpfLoader,
};
use log::info;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;

const PIN_DIR: &str = "/sys/fs/bpf/ebpf-aya";

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
            KeyValue::new("service.name", "ebpf-lifecycle"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    // bpffs subdirectory to pin into (idempotent)
    std::fs::create_dir_all(PIN_DIR).ok();

    // map_pin_path pins (or REUSES) the program's maps under PIN_DIR
    let mut ebpf = EbpfLoader::new()
        .map_pin_path("EVENTS", std::path::PathBuf::from(format!("{PIN_DIR}/EVENTS")))
        .load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/lifecycle")))?;

    let link_pin = format!("{PIN_DIR}/svc_link");
    if Path::new(&link_pin).exists() {
        info!("link already pinned — reusing the running program and pinned map (state continues)");
    } else {
        let prog: &mut TracePoint = ebpf.program_mut("count").unwrap().try_into()?;
        prog.load()?;
        let id = prog.attach("syscalls", "sys_enter_getpid")?;
        // turn the attachment into a pinnable FdLink and pin it — now the
        // program stays attached after THIS process exits.
        let link = prog.take_link(id)?;
        let fd_link: FdLink = link.try_into()?;
        fd_link.pin(&link_pin)?;
        info!("attached and pinned link at {link_pin}");
    }

    let provider = init_otel()?;
    let counter = global::meter("ebpf-lifecycle").u64_counter("ebpf_service_events_total").build();

    let events: Array<_, u64> = Array::try_from(ebpf.map("EVENTS").unwrap())?;
    let mut last = events.get(&0, 0).unwrap_or(0);
    println!("starting count = {last} (continues across runs if pinned)");

    for _ in 0..8 {
        for _ in 0..100 { unsafe { libc::getpid() }; }
        tokio::time::sleep(Duration::from_secs(1)).await;
        let now = events.get(&0, 0).unwrap_or(0);
        counter.add(now.saturating_sub(last), &[]);
        println!("count = {now}");
        last = now;
    }

    println!("exiting — link + map stay PINNED, program keeps counting");
    tokio::time::sleep(Duration::from_secs(2)).await;
    provider.shutdown()?;
    Ok(())
}
