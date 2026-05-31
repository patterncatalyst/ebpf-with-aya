//! kfunc-task — drive the kfunc lookup: set a target pid, trigger the
//! tracepoint (by calling getpid), and read back found/missing tallies.
//! Phase 1 uses our own (real) pid; phase 2 a bogus pid. Exports
//! ebpf_task_lookups_total{result=found|missing}.
use std::time::Duration;

use aya::{
    maps::{Array, HashMap},
    programs::TracePoint,
    Ebpf,
};
use log::info;
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;

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
            KeyValue::new("service.name", "ebpf-kfunc-task"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn read_tallies(ebpf: &mut Ebpf) -> anyhow::Result<(u64, u64)> {
    let r: HashMap<_, u32, u64> = HashMap::try_from(ebpf.map_mut("RESULT").unwrap())?;
    Ok((r.get(&0, 0).unwrap_or(0), r.get(&1, 0).unwrap_or(0)))
}

async fn phase(ebpf: &mut Ebpf, lookups: &Counter<u64>, target: u32, label: &str) -> anyhow::Result<()> {
    {
        let mut cfg: Array<_, u32> = Array::try_from(ebpf.map_mut("CONFIG").unwrap())?;
        cfg.set(0, target, 0)?;
    }
    let (bf, bm) = read_tallies(ebpf)?;
    for _ in 0..50 {
        unsafe { libc::getpid() }; // fires sys_enter_getpid -> our tracepoint
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let (f, m) = read_tallies(ebpf)?;
    let (df, dm) = (f - bf, m - bm);
    println!("phase {label}: target_pid={target}  found+={df}  missing+={dm}");
    lookups.add(df, &[KeyValue::new("result", "found")]);
    lookups.add(dm, &[KeyValue::new("result", "missing")]);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/kfunc-task")))?;
    let prog: &mut TracePoint = ebpf.program_mut("lookup").unwrap().try_into()?;
    prog.load()?;
    prog.attach("syscalls", "sys_enter_getpid")?;
    info!("attached; looking up tasks by pid via bpf_task_from_pid");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-kfunc-task");
    let lookups: Counter<u64> = meter.u64_counter("ebpf_task_lookups_total").build();

    let me = std::process::id();
    phase(&mut ebpf, &lookups, me, "real (our own pid)").await?;
    phase(&mut ebpf, &lookups, 0x7FFF_FFFE, "bogus pid").await?;

    tokio::time::sleep(Duration::from_secs(2)).await; // flush metrics
    provider.shutdown()?;
    Ok(())
}
