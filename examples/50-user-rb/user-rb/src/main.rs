//! user-rb — PRODUCER side. Submit a stream of samples into the user ring
//! buffer, trigger the consumer to drain (by calling getpid), and read back
//! the aggregate the callback built. Exports ebpf_userrb_messages_total + sum.
//!
//! EXPERIMENTAL: Aya's user-space UserRingBuf producer API is still settling;
//! the reserve/submit calls here are the expected shape. See README.
use std::time::Duration;

use aya::{
    maps::{HashMap, UserRingBuf},
    programs::TracePoint,
    Ebpf,
};
use log::info;
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use user_rb_common::Sample;

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
            KeyValue::new("service.name", "ebpf-user-rb"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/user-rb")))?;
    let prog: &mut TracePoint = ebpf.program_mut("drain_it").unwrap().try_into()?;
    prog.load()?;
    prog.attach("syscalls", "sys_enter_getpid")?;
    info!("consumer attached; producing samples into the user ring buffer");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-user-rb");
    let msgs: Counter<u64> = meter.u64_counter("ebpf_userrb_messages_total").build();
    let summ: Counter<u64> = meter.u64_counter("ebpf_userrb_value_sum_total").build();

    // PRODUCER: submit a stream of samples (API shape; still settling in Aya)
    let mut urb = UserRingBuf::try_from(ebpf.map_mut("USER_RB").unwrap())?;
    for i in 1u64..=1000 {
        if let Ok(mut entry) = urb.reserve(std::mem::size_of::<Sample>()) {
            let s = Sample { value: i };
            entry.copy_from_slice(unsafe {
                std::slice::from_raw_parts(&s as *const _ as *const u8, std::mem::size_of::<Sample>())
            });
            entry.submit()?;
        }
        msgs.add(1, &[]);
        summ.add(i, &[]);
        unsafe { libc::getpid() };           // trigger the consumer to drain
        if i % 100 == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    // read back what the callback aggregated
    let agg: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("AGG").unwrap())?;
    let count = agg.get(&0, 0).unwrap_or(0);
    let sum = agg.get(&1, 0).unwrap_or(0);
    println!("consumer drained: count={count} sum={sum} (expected count=1000, sum=500500)");
    tokio::time::sleep(Duration::from_secs(2)).await; // let metrics flush
    provider.shutdown()?;
    Ok(())
}
