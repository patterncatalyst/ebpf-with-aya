//! scx-watch — attach a sched_switch probe and export per-CPU context-switch
//! counts as ebpf_ctxsw_total{cpu}. Used to observe a sched_ext scheduler
//! (scx_simple) running the machine.
use std::collections::HashMap as Std;
use std::time::Duration;

use aya::{maps::HashMap, programs::TracePoint, Ebpf};
use log::info;
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;

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
            KeyValue::new("service.name", "ebpf-scx-watch"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/scx-watch")))?;
    let prog: &mut TracePoint = ebpf.program_mut("on_switch").unwrap().try_into()?;
    prog.load()?;
    prog.attach("sched", "sched_switch")?;
    info!("watching context switches per CPU (observe the active scheduler)");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-scx-watch");
    let ctxsw: Counter<u64> = meter.u64_counter("ebpf_ctxsw_total").build();
    let map: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("CTXSW").unwrap())?;
    let mut last: Std<u32, u64> = Std::new();

    println!("{:<6} {:>14}", "CPU", "ctx-switches");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                for res in map.iter() {
                    let (cpu, total) = res?;
                    let prev = last.get(&cpu).copied().unwrap_or(0);
                    if total > prev {
                        ctxsw.add(total - prev, &[KeyValue::new("cpu", cpu.to_string())]);
                        last.insert(cpu, total);
                    }
                    println!("{:<6} {:>14}", cpu, total);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
