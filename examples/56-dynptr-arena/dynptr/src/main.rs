//! dynptr — read the variable-length records the producer emits and report each
//! record's actual length, counting them. Exports ebpf_dynptr_records_total.
use std::time::Duration;

use aya::{maps::RingBuf, programs::TracePoint, Ebpf};
use dynptr_common::Record;
use log::info;
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use tokio::io::unix::AsyncFd;

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
            KeyValue::new("service.name", "ebpf-dynptr"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/dynptr")))?;
    let prog: &mut TracePoint = ebpf.program_mut("emit").unwrap().try_into()?;
    prog.load()?;
    prog.attach("syscalls", "sys_enter_getpid")?;
    info!("emitting variable-length records via the ring buffer");

    let provider = init_otel()?;
    let recs: Counter<u64> = global::meter("ebpf-dynptr")
        .u64_counter("ebpf_dynptr_records_total").build();

    let ring = RingBuf::try_from(ebpf.take_map("RB").unwrap())?;
    let mut afd = AsyncFd::new(ring)?;
    // drive a stream of getpid so records flow
    tokio::spawn(async { loop { unsafe { libc::getpid() }; tokio::time::sleep(Duration::from_millis(20)).await; } });

    println!("{:>8} {:>8}", "pid", "len");
    let mut seen = 0u64;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            g = afd.readable_mut() => {
                let mut guard = g?;
                let ring = guard.get_inner_mut();
                while let Some(item) = ring.next() {
                    let r: &Record = unsafe { &*(item.as_ptr() as *const Record) };
                    println!("{:>8} {:>8}", r.pid, r.len);   // lengths differ per record
                    recs.add(1, &[]);
                    seen += 1;
                }
                guard.clear_ready();
                if seen >= 40 { break; }
            }
        }
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    provider.shutdown()?;
    Ok(())
}
