//! pinctl — demonstrate BPF object lifecycle. Subcommands:
//!   load    load + attach + pin link & map to bpffs, then EXIT (program lives on)
//!   read    open the pinned map from a fresh process and print/export the count
//!   detach  remove the pins (drops the references -> program detaches)
//!
//! UNVERIFIED: the Aya pinning API used here (HashMap::pinned + map_pin_path,
//! take_link, FdLink::try_from/pin, MapData::from_pin) must be confirmed.
use std::time::Duration;

use aya::{
    maps::{HashMap, Map, MapData},
    programs::{links::FdLink, TracePoint},
    EbpfLoader,
};

const DIR: &str = "/sys/fs/bpf/ebpf-aya";
const MAP_PIN: &str = "/sys/fs/bpf/ebpf-aya/EXECS";
const LINK_PIN: &str = "/sys/fs/bpf/ebpf-aya/execs_link";

fn cmd_load() -> anyhow::Result<()> {
    std::fs::create_dir_all(DIR)?;
    // map_pin_path + the map's `pinned` flag -> EXECS is pinned at DIR/EXECS
    let mut ebpf = EbpfLoader::new()
        .map_pin_path(DIR)
        .load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/pin-demo")))?;
    let prog: &mut TracePoint = ebpf.program_mut("count_exec").unwrap().try_into()?;
    prog.load()?;
    let link_id = prog.attach("syscalls", "sys_enter_execve")?;
    // pin the LINK so the attachment survives us
    let link = prog.take_link(link_id)?;
    let fd_link: FdLink = link.try_into()?;
    let _ = std::fs::remove_file(LINK_PIN);
    fd_link.pin(LINK_PIN)?;
    println!("loaded, attached, pinned to {DIR} — exiting; the program keeps counting");
    Ok(())
}

fn read_count() -> anyhow::Result<u64> {
    let map = Map::HashMap(MapData::from_pin(MAP_PIN)?);
    let execs: HashMap<_, u32, u64> = HashMap::try_from(map)?;
    Ok(execs.get(&0, 0).unwrap_or(0))
}

async fn cmd_read() -> anyhow::Result<()> {
    let n = read_count()?;
    println!("execs so far (from pinned map, no loader running): {n}");
    // export the persisted counter so any reader can feed Grafana
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4318".to_string());
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http().with_endpoint(format!("{endpoint}/v1/metrics")).build()?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(
        exporter, opentelemetry_sdk::runtime::Tokio).with_interval(Duration::from_secs(1)).build();
    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", "ebpf-pin-demo"),
            opentelemetry::KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    opentelemetry::global::set_meter_provider(provider.clone());
    let c = opentelemetry::global::meter("ebpf-pin-demo").u64_counter("ebpf_pinned_execs_total").build();
    c.add(n, &[]);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    provider.shutdown()?;
    Ok(())
}

fn cmd_detach() -> anyhow::Result<()> {
    let _ = std::fs::remove_file(LINK_PIN);
    let _ = std::fs::remove_file(MAP_PIN);
    println!("removed pins — references dropped, program detaches");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    match std::env::args().nth(1).as_deref() {
        Some("load") => cmd_load(),
        Some("read") => cmd_read().await,
        Some("detach") => cmd_detach(),
        _ => {
            eprintln!("usage: pinctl <load|read|detach>");
            std::process::exit(2);
        }
    }
}
