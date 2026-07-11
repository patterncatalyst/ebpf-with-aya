//! contrace — cgroup-scoped container observer.
//!
//! Usage: contrace [CGROUP_ID] [LABEL]
//!   CGROUP_ID  the container's cgroup id to scope to (0 or omitted = all)
//!   LABEL      a friendly name used as the `container` metric label
//!
//! Find a container's cgroup id on the VM (best effort):
//!   cgpath=$(podman inspect --format '{{.State.CgroupPath}}' NAME)
//!   stat -c %i "/sys/fs/cgroup${cgpath}"
use std::time::Duration;

use aya::{maps::{Array, RingBuf}, programs::TracePoint, Ebpf};
use aya_log::EbpfLogger;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use contrace_common::{ContainerEvent, COMM_LEN};

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
            KeyValue::new("service.name", "ebpf-contrace"),
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
    let target_cgroup: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let label = std::env::args().nth(2).unwrap_or_else(|| if target_cgroup == 0 { "all".into() } else { "container".into() });

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/contrace")))?;

    // Write the target cgroup id into the config map before attaching.
    {
        let mut cfg: Array<_, u64> = Array::try_from(ebpf.map_mut("TARGET_CGROUP").unwrap())?;
        cfg.set(0, target_cgroup, 0)?;
    }

    let tp: &mut TracePoint = ebpf.program_mut("sys_enter_openat").unwrap().try_into()?;
    tp.load()?;
    tp.attach("syscalls", "sys_enter_openat")?;
    if target_cgroup == 0 {
        info!("contrace attached (scoping OFF — tracing all cgroups)");
    } else {
        info!("contrace attached, scoped to cgroup {target_cgroup} (label={label})");
    }
    if let Err(e) = EbpfLogger::init(&mut ebpf) { warn!("aya-log init failed: {e}"); }

    let provider = init_otel()?;
    let meter = global::meter("ebpf-contrace");
    let counter = meter.u64_counter("ebpf_events_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<8} {:<20} {:<16} {}", "PID", "CGROUP", "COMM", "FILE");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<ContainerEvent>() { continue; }
                    let ev: ContainerEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const ContainerEvent) };
                    println!("{:<8} {:<20} {:<16} {}", ev.pid, ev.cgroup, cstr(&ev.comm[..COMM_LEN]), cstr(&ev.filename));
                    counter.add(1, &[KeyValue::new("program", "contrace"), KeyValue::new("container", label.clone())]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
