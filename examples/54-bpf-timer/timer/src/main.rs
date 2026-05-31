//! timer — load the in-kernel aggregator, hold the map open (required for
//! timers), arm it once (by triggering an execve), drive a getpid stream, and
//! read the kernel-computed per-second rate. Exports ebpf_timer_events_per_sec.
use std::sync::{atomic::{AtomicU64, Ordering}, Arc};
use std::time::Duration;

use aya::{maps::Array, programs::TracePoint, Ebpf};
use log::info;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use timer_common::Slot;

fn init_otel() -> anyhow::Result<opentelemetry_sdk::metrics::SdkMeterProvider> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4318".to_string());
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http().with_endpoint(format!("{endpoint}/v1/metrics")).build()?;
    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(
        exporter, opentelemetry_sdk::runtime::Tokio).with_interval(Duration::from_secs(2)).build();
    let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            KeyValue::new("service.name", "ebpf-bpf-timer"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/timer")))?;
    for (prog, tp) in [("count", "sys_enter_getpid"), ("arm", "sys_enter_execve")] {
        let p: &mut TracePoint = ebpf.program_mut(prog).unwrap().try_into()?;
        p.load()?;
        p.attach("syscalls", tp)?;
    }
    info!("loaded; arming the in-kernel timer");

    let provider = init_otel()?;
    let rate = Arc::new(AtomicU64::new(0));
    let r2 = rate.clone();
    let _gauge = global::meter("ebpf-bpf-timer")
        .u64_observable_gauge("ebpf_timer_events_per_sec")
        .with_callback(move |obs| obs.observe(r2.load(Ordering::Relaxed), &[]))
        .build();

    // arm the timer once: cause an execve so the `arm` program runs
    let _ = std::process::Command::new("true").status();

    println!("{:>8}  {}", "rate/s", "(kernel-computed, via self-rescheduling bpf_timer)");
    for _ in 0..20 {
        // drive ~200 getpid/s for one second
        for _ in 0..200 {
            unsafe { libc::getpid() };
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let slots: Array<_, Slot> = Array::try_from(ebpf.map_mut("SLOTS").unwrap())?;
        let r = slots.get(&0, 0).map(|s| s.rate).unwrap_or(0);
        rate.store(r, Ordering::Relaxed);
        println!("{:>8}", r);
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    provider.shutdown()?;
    Ok(())
}
