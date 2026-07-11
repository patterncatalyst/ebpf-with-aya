//! bashreadline — user space for the readline uretprobe. Attaches to the
//! `readline` symbol in the bash binary (override with BASH_PATH / READLINE_LIB
//! if your distro puts readline in libreadline.so). Prints commands typed at
//! interactive bash prompts; exports ebpf_events_total{program="bashreadline"}.
use std::time::Duration;

use aya::{maps::RingBuf, programs::{UProbe, uprobe::UProbeScope}, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use bashreadline_common::{ReadlineEvent, COMM_LEN};

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
            KeyValue::new("service.name", "ebpf-bashreadline"),
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
    // Where readline() lives. On Fedora the symbol is usually in the bash
    // binary; if not, point at libreadline (e.g. /usr/lib64/libreadline.so.8).
    let target = std::env::var("READLINE_LIB")
        .or_else(|_| std::env::var("BASH_PATH"))
        .unwrap_or_else(|_| "/usr/bin/bash".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/bashreadline")))?;

    let prog: &mut UProbe = ebpf.program_mut("readline_ret").unwrap().try_into()?;
    prog.load()?;
    // attach(location, target, scope): symbol "readline", whole-system.
    prog.attach("readline", &target, UProbeScope::AllProcesses)?;
    info!("uretprobe attached to readline in {target}");
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let provider = init_otel()?;
    let meter = global::meter("ebpf-bashreadline");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<8} {}", "PID", "UID", "COMMAND");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<ReadlineEvent>() { continue; }
                    let ev: ReadlineEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const ReadlineEvent) };
                    let line = cstr(&ev.line);
                    if line.is_empty() { continue; }
                    println!("{:<8} {:<8} {}", ev.pid, ev.uid, line);
                    counter.add(1, &[KeyValue::new("program", "bashreadline")]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
