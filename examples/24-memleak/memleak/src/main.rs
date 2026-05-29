//! memleak — attaches malloc/calloc/free uprobes to libc, filtered to one pid,
//! and after a sampling window reports OUTSTANDING allocations grouped by their
//! allocation stack (the candidate leaks). User frames are shown as hex (wire
//! in blazesym for names). Exports memleak_outstanding_bytes.
//!
//! Usage: memleak PID [LIBC_PATH] [DURATION_SECS]
use std::collections::HashMap as Std;
use std::time::Duration;

use aya::{
    maps::{Array, HashMap as BpfHashMap, StackTraceMap},
    programs::UProbe,
    Ebpf,
};
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use memleak_common::AllocInfo;

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
            KeyValue::new("service.name", "ebpf-memleak"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let pid: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("usage: memleak PID [LIBC_PATH] [DURATION_SECS]"))?;
    let libc = std::env::args().nth(2).unwrap_or_else(|| "/usr/lib64/libc.so.6".to_string());
    let secs: u64 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(15);

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/memleak")))?;

    // Scope to the target pid before attaching.
    {
        let mut t: Array<_, u32> = Array::try_from(ebpf.map_mut("TARGET_PID").unwrap())?;
        t.set(0, pid, 0)?;
    }

    // (program, symbol, is_uretprobe)
    let attach = |ebpf: &mut Ebpf, prog: &str, sym: &str| -> anyhow::Result<()> {
        let p: &mut UProbe = ebpf.program_mut(prog).unwrap().try_into()?;
        p.load()?;
        p.attach(Some(sym), 0, &libc, None)?;
        Ok(())
    };
    attach(&mut ebpf, "malloc_enter", "malloc")?;
    attach(&mut ebpf, "malloc_exit", "malloc")?;
    attach(&mut ebpf, "calloc_enter", "calloc")?;
    attach(&mut ebpf, "calloc_exit", "calloc")?;
    attach(&mut ebpf, "free_enter", "free")?;
    info!("memleak watching pid {pid} via {libc} for {secs}s...");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-memleak");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => { info!("stopped early"); }
        _ = tokio::time::sleep(Duration::from_secs(secs)) => {}
    }

    // Group outstanding allocations by stack id.
    let allocs: BpfHashMap<_, u64, AllocInfo> = BpfHashMap::try_from(ebpf.map("ALLOCS").unwrap())?;
    let stacks = StackTraceMap::try_from(ebpf.map("STACKS").unwrap())?;
    let mut by_stack: Std<i32, (u64, u64)> = Std::new(); // stackid -> (bytes, count)
    for item in allocs.iter() {
        if let Ok((_ptr, info)) = item {
            let e = by_stack.entry(info.stackid).or_insert((0, 0));
            e.0 += info.size; e.1 += 1;
        }
    }

    let mut rows: Vec<(i32, u64, u64)> = by_stack.iter().map(|(k, v)| (*k, v.0, v.1)).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    let mut total_bytes = 0u64;
    println!("\noutstanding allocations (candidate leaks), by stack:");
    for (stackid, bytes, count) in &rows {
        total_bytes += bytes;
        println!("  {bytes} bytes in {count} allocations:");
        if *stackid >= 0 {
            if let Ok(trace) = stacks.get(&(*stackid as u32), 0) {
                for frame in trace.frames() {
                    println!("      {:#x}", frame.ip); // user frame; symbolize with blazesym
                }
            }
        }
    }
    println!("total outstanding: {total_bytes} bytes");

    let gauge = meter.u64_gauge("memleak_outstanding_bytes").build();
    gauge.record(total_bytes, &[KeyValue::new("pid", pid.to_string())]);
    provider.shutdown()?;
    Ok(())
}
