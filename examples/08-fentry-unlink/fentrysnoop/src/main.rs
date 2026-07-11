//! fentrysnoop — user space for the fentry/fexit vfs_unlink programs.
//!
//! Loads + attaches both programs (they share the INFLIGHT map), drains
//! completed UnlinkEvents from the ring buffer, prints them with success/
//! failure, and exports `ebpf_events_total{program="fentrysnoop",result=...}`.

use std::time::Duration;

use aya::{
    maps::RingBuf,
    programs::{FEntry, FExit},
    Btf, Ebpf,
};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use fentrysnoop_common::{UnlinkEvent, COMM_LEN};

fn init_otel() -> anyhow::Result<opentelemetry_sdk::metrics::SdkMeterProvider> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4318".to_string());
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_endpoint(format!("{endpoint}/v1/metrics"))
        .build()?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(
        exporter,
        opentelemetry_sdk::runtime::Tokio,
    )
    .with_interval(Duration::from_secs(2))
    .build();
    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            KeyValue::new("service.name", "ebpf-fentrysnoop"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ]))
        .build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn cstr(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/fentrysnoop"
    )))?;
    // fentry/fexit need the kernel's BTF to resolve the target function type.
    let btf = Btf::from_sys_fs()?;

    let enter: &mut FEntry = ebpf.program_mut("vfs_unlink_enter").unwrap().try_into()?;
    enter.load("vfs_unlink", &btf)?;
    enter.attach()?;

    let exit: &mut FExit = ebpf.program_mut("vfs_unlink_exit").unwrap().try_into()?;
    exit.load("vfs_unlink", &btf)?;
    exit.attach()?;

    // aya-log 0.3's EbpfLogger::init take_map()s AYA_LOGS out of the object, so
    // it must run AFTER the programs are loaded (they reference that map).
    if let Err(e) = EbpfLogger::init(&mut ebpf) {
        warn!("failed to initialize aya-log: {e}");
    }
    info!("fentry+fexit attached to vfs_unlink; watching for unlinks");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-fentrysnoop");
    let counter = meter.u64_counter("ebpf_events_total").build();

    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<8} {:<6} {:<16} {}", "PID", "UID", "RET", "COMM", "FILE");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<UnlinkEvent>() { continue; }
                    let ev: UnlinkEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const UnlinkEvent) };
                    let result = if ev.ret == 0 { "ok" } else { "fail" };
                    println!("{:<8} {:<8} {:<6} {:<16} {}", ev.pid, ev.uid, ev.ret, cstr(&ev.comm[..COMM_LEN]), cstr(&ev.filename));
                    counter.add(1, &[
                        KeyValue::new("program", "fentrysnoop"),
                        KeyValue::new("result", result),
                    ]);
                }
            }
        }
    }

    provider.shutdown()?;
    Ok(())
}
