//! execsnoop — user space for the execve tracepoint. Reassembles argv from the
//! fixed slots, prints "PID UID COMM CMDLINE", exports
//! ebpf_events_total{program="execsnoop"}.
use std::time::Duration;

use aya::{maps::RingBuf, programs::TracePoint, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use execsnoop_common::{ExecEvent, ARG_LEN, COMM_LEN, MAX_ARGS};

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
            KeyValue::new("service.name", "ebpf-execsnoop"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

fn cmdline(ev: &ExecEvent) -> String {
    let n = (ev.args_count as usize).min(MAX_ARGS);
    let mut parts: Vec<String> = Vec::with_capacity(n);
    for i in 0..n {
        let s = cstr(&ev.args[i][..ARG_LEN]);
        if !s.is_empty() { parts.push(s); }
    }
    if parts.is_empty() { cstr(&ev.filename) } else { parts.join(" ") }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/execsnoop")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let tp: &mut TracePoint = ebpf.program_mut("sys_enter_execve").unwrap().try_into()?;
    tp.load()?;
    tp.attach("syscalls", "sys_enter_execve")?;
    info!("execsnoop attached to syscalls:sys_enter_execve");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-execsnoop");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<8} {:<16} {}", "PID", "UID", "COMM", "CMDLINE");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<ExecEvent>() { continue; }
                    let ev: ExecEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const ExecEvent) };
                    println!("{:<8} {:<8} {:<16} {}", ev.pid, ev.uid, cstr(&ev.comm[..COMM_LEN]), cmdline(&ev));
                    counter.add(1, &[KeyValue::new("program", "execsnoop")]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
