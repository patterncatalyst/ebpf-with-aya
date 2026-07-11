//! unlinksnoop — user space for the vfs_unlink kprobe.
//!
//! Loads the program, attaches the kprobe to `vfs_unlink`, drains the ring
//! buffer of UnlinkEvents, prints each one, and exports `ebpf_events_total`
//! (labelled program="unlinksnoop") to the observability stack.
//!
//! Run on the target VM under sudo. Point at the stack with
//! OTEL_EXPORTER_OTLP_ENDPOINT (default http://127.0.0.1:4318).

use std::time::Duration;

use aya::{maps::RingBuf, programs::KProbe, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use unlinksnoop_common::{UnlinkEvent, COMM_LEN};

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
            KeyValue::new("service.name", "ebpf-unlinksnoop"),
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
        "/unlinksnoop"
    )))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) {
        warn!("failed to initialize aya-log: {e}");
    }

    let program: &mut KProbe = ebpf.program_mut("vfs_unlink").unwrap().try_into()?;
    program.load()?;
    // Attach to the kernel function vfs_unlink (offset 0 = entry).
    program.attach("vfs_unlink", 0)?;
    info!("kprobe attached to vfs_unlink; watching for unlinks");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-unlinksnoop");
    let counter = meter.u64_counter("ebpf_events_total").build();

    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<8} {:<16} {}", "PID", "UID", "COMM", "FILE");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                // Drain whatever the kernel has produced since last tick.
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<UnlinkEvent>() { continue; }
                    // SAFETY: the kernel wrote exactly an UnlinkEvent (#[repr(C)]);
                    // read_unaligned avoids any alignment assumption on the ring slot.
                    let ev: UnlinkEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const UnlinkEvent) };
                    let comm = cstr(&ev.comm[..COMM_LEN]);
                    let file = cstr(&ev.filename);
                    println!("{:<8} {:<8} {:<16} {}", ev.pid, ev.uid, comm, file);
                    counter.add(1, &[KeyValue::new("program", "unlinksnoop")]);
                }
            }
        }
    }

    provider.shutdown()?;
    Ok(())
}
