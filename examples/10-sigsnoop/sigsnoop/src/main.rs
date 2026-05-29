//! sigsnoop — user space for the kill tracepoint. Drains SignalEvents, maps the
//! signal number to a name, prints, exports
//! ebpf_events_total{program="sigsnoop",signal=NAME}.
use std::time::Duration;

use aya::{maps::RingBuf, programs::TracePoint, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use sigsnoop_common::{SignalEvent, COMM_LEN};

fn sig_name(sig: i32) -> &'static str {
    match sig {
        1 => "SIGHUP", 2 => "SIGINT", 3 => "SIGQUIT", 9 => "SIGKILL",
        11 => "SIGSEGV", 13 => "SIGPIPE", 15 => "SIGTERM", 17 => "SIGCHLD",
        18 => "SIGCONT", 19 => "SIGSTOP", _ => "SIG?",
    }
}

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
            KeyValue::new("service.name", "ebpf-sigsnoop"),
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
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/sigsnoop")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let kill: &mut TracePoint = ebpf.program_mut("sys_enter_kill").unwrap().try_into()?;
    kill.load()?;
    kill.attach("syscalls", "sys_enter_kill")?;
    info!("sigsnoop attached to syscalls:sys_enter_kill");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-sigsnoop");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<16} {:<10} {}", "SENDER", "COMM", "SIGNAL", "TARGET");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<SignalEvent>() { continue; }
                    let ev: SignalEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const SignalEvent) };
                    let name = sig_name(ev.sig);
                    println!("{:<8} {:<16} {:<10} {}", ev.sender_pid, cstr(&ev.comm[..COMM_LEN]), name, ev.target_pid);
                    counter.add(1, &[KeyValue::new("program", "sigsnoop"), KeyValue::new("signal", name)]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
