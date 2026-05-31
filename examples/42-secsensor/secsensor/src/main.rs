//! secsensor — attach exec/ptrace/setuid tracepoints, classify each event by
//! type and severity, and export ebpf_sec_events_total{type,severity}.
use std::time::Duration;

use aya::{maps::RingBuf, programs::TracePoint, Ebpf};
use log::info;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use secsensor_common::{SecEvent, ET_EXEC, ET_PTRACE, ET_SETUID};

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
            KeyValue::new("service.name", "ebpf-secsensor"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

fn classify(etype: u32) -> (&'static str, &'static str) {
    match etype {
        ET_EXEC => ("exec", "info"),
        ET_PTRACE => ("ptrace", "warning"),
        ET_SETUID => ("setuid", "warning"),
        _ => ("other", "info"),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/secsensor")))?;

    for (prog_name, tp) in [
        ("on_exec", "sys_enter_execve"),
        ("on_ptrace", "sys_enter_ptrace"),
        ("on_setuid", "sys_enter_setuid"),
    ] {
        let prog: &mut TracePoint = ebpf.program_mut(prog_name).unwrap().try_into()?;
        prog.load()?;
        prog.attach("syscalls", tp)?;
    }
    info!("security sensor attached — exec/ptrace/setuid streaming as one event feed");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-secsensor");
    let events = meter.u64_counter("ebpf_sec_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<SecEvent>() { continue; }
                    let ev: SecEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const SecEvent) };
                    let (kind, severity) = classify(ev.etype);
                    println!("[{severity:<7}] {kind:<7} pid={:<7} comm={}", ev.pid, cstr(&ev.comm));
                    events.add(1, &[KeyValue::new("type", kind), KeyValue::new("severity", severity)]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
