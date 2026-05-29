//! opensnoop — user space for the openat tracepoints. Attaches enter+exit,
//! drains OpenEvents, prints them, exports
//! ebpf_events_total{program="opensnoop",result="ok|err"}.
use std::time::Duration;

use aya::{maps::RingBuf, programs::TracePoint, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opensnoop_common::{OpenEvent, COMM_LEN};

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
            KeyValue::new("service.name", "ebpf-opensnoop"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/opensnoop")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let enter: &mut TracePoint = ebpf.program_mut("sys_enter_openat").unwrap().try_into()?;
    enter.load()?;
    enter.attach("syscalls", "sys_enter_openat")?;
    let exit: &mut TracePoint = ebpf.program_mut("sys_exit_openat").unwrap().try_into()?;
    exit.load()?;
    exit.attach("syscalls", "sys_exit_openat")?;
    info!("opensnoop attached to syscalls:sys_{{enter,exit}}_openat");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-opensnoop");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<8} {:<6} {:<16} {}", "PID", "UID", "RET", "COMM", "FILE");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<OpenEvent>() { continue; }
                    let ev: OpenEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const OpenEvent) };
                    let result = if ev.ret >= 0 { "ok" } else { "err" };
                    println!("{:<8} {:<8} {:<6} {:<16} {}", ev.pid, ev.uid, ev.ret, cstr(&ev.comm[..COMM_LEN]), cstr(&ev.filename));
                    counter.add(1, &[KeyValue::new("program", "opensnoop"), KeyValue::new("result", result)]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
