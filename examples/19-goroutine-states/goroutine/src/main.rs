//! goroutine — attaches a uprobe to runtime.casgstatus in a Go binary
//! (argv[1], default /home/fedora/target-go) and reports goroutine state
//! transitions by name. Exports ebpf_events_total{program="goroutine",state}.
use std::time::Duration;

use aya::{maps::RingBuf, programs::{UProbe, uprobe::UProbeScope}, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use goroutine_common::{GoStateEvent, COMM_LEN};

// Go runtime goroutine status values (src/runtime/runtime2.go).
fn state_name(s: u32) -> &'static str {
    match s {
        0 => "idle", 1 => "runnable", 2 => "running", 3 => "syscall",
        4 => "waiting", 5 => "moribund", 6 => "dead", 7 => "enqueue",
        8 => "copystack", 9 => "preempted", _ => "other",
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
            KeyValue::new("service.name", "ebpf-goroutine"),
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
    let bin = std::env::args().nth(1).unwrap_or_else(|| "/home/fedora/target-go".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/goroutine")))?;
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let prog: &mut UProbe = ebpf.program_mut("casgstatus").unwrap().try_into()?;
    prog.load()?;
    // Attach ONLY a uprobe (entry). Never a uretprobe on Go — see the chapter.
    prog.attach("runtime.casgstatus", &bin, UProbeScope::AllProcesses)?;
    info!("uprobe attached to runtime.casgstatus in {bin}");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-goroutine");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<16} {}", "PID(M)", "COMM", "NEW STATE");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<GoStateEvent>() { continue; }
                    let ev: GoStateEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const GoStateEvent) };
                    let st = state_name(ev.newstate);
                    println!("{:<8} {:<16} {}", ev.pid, cstr(&ev.comm[..COMM_LEN]), st);
                    counter.add(1, &[KeyValue::new("program", "goroutine"), KeyValue::new("state", st)]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
