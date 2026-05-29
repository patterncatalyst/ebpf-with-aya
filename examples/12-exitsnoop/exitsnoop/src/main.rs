//! exitsnoop — user space for the exit_group tracepoint. Decodes the status,
//! prints "PID COMM CODE STATUS", exports
//! ebpf_events_total{program="exitsnoop",status="ok|nonzero"}.
use std::time::Duration;

use aya::{maps::RingBuf, programs::TracePoint, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use exitsnoop_common::{ExitEvent, COMM_LEN};

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
            KeyValue::new("service.name", "ebpf-exitsnoop"),
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
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/exitsnoop")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let tp: &mut TracePoint = ebpf.program_mut("sys_enter_exit_group").unwrap().try_into()?;
    tp.load()?;
    tp.attach("syscalls", "sys_enter_exit_group")?;
    info!("exitsnoop attached to syscalls:sys_enter_exit_group");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-exitsnoop");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<16} {:<6} {}", "PID", "COMM", "CODE", "STATUS");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<ExitEvent>() { continue; }
                    let ev: ExitEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const ExitEvent) };
                    // exit_group's arg is the raw status the program passed
                    // (e.g. exit(3) -> 3). The exit code is its low 8 bits.
                    // (This differs from task_struct->exit_code, which packs
                    // the code in the HIGH byte and a signal in the low byte.)
                    let exit_code = ev.code & 0xff;
                    let status = if exit_code == 0 { "ok" } else { "nonzero" };
                    println!("{:<8} {:<16} {:<6} {}", ev.pid, cstr(&ev.comm[..COMM_LEN]), exit_code, status);
                    counter.add(1, &[KeyValue::new("program", "exitsnoop"), KeyValue::new("status", status)]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
