//! energy — attribute estimated power to processes by their CPU-time share.
//!
//! Reads per-task on-CPU nanoseconds from the eBPF USAGE map, aggregates by
//! comm, and multiplies each comm's CPU-time share by the system power. System
//! power comes from RAPL (/sys/class/powercap/.../energy_uj) when available; on
//! VMs that don't expose RAPL it falls back to ENERGY_TDP_WATTS (default 15W)
//! as a flat model. Exports estimated_power_watts{comm} + system_power_watts.
use std::collections::HashMap as Std;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use aya::maps::HashMap as BpfHashMap;
use aya::Ebpf;
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use energy_common::{TaskStat, COMM_LEN};

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
            KeyValue::new("service.name", "ebpf-energy"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

// RAPL "package-0" energy counter in microjoules (monotonic, wraps).
fn rapl_uj() -> Option<u64> {
    for p in [
        "/sys/class/powercap/intel-rapl:0/energy_uj",
        "/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj",
    ] {
        if let Ok(s) = std::fs::read_to_string(p) {
            if let Ok(v) = s.trim().parse::<u64>() { return Some(v); }
        }
    }
    None
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let tdp: f64 = std::env::var("ENERGY_TDP_WATTS").ok().and_then(|s| s.parse().ok()).unwrap_or(15.0);

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/energy")))?;
    let tp: &mut aya::programs::TracePoint = ebpf.program_mut("sched_switch").unwrap().try_into()?;
    tp.load()?;
    tp.attach("sched", "sched_switch")?;
    if rapl_uj().is_some() {
        info!("energy: RAPL available — power from hardware energy counter");
    } else {
        warn!("energy: no RAPL (typical on VMs) — modelling system power as ENERGY_TDP_WATTS={tdp}W");
    }

    // Snapshot of comm -> watts for the OTLP gauge callback.
    let snap: Arc<Mutex<Std<String, f64>>> = Arc::new(Mutex::new(Std::new()));
    let sys_w: Arc<Mutex<f64>> = Arc::new(Mutex::new(0.0));

    let provider = init_otel()?;
    let meter = global::meter("ebpf-energy");
    {
        let snap = snap.clone();
        let _g = meter.f64_observable_gauge("ebpf_estimated_power_watts")
            .with_callback(move |obs| {
                for (comm, w) in snap.lock().unwrap().iter() {
                    obs.observe(*w, &[KeyValue::new("comm", comm.clone())]);
                }
            }).build();
    }
    {
        let sys_w = sys_w.clone();
        let _g = meter.f64_observable_gauge("ebpf_system_power_watts")
            .with_callback(move |obs| { obs.observe(*sys_w.lock().unwrap(), &[]); })
            .build();
    }

    let usage: BpfHashMap<_, u32, TaskStat> = BpfHashMap::try_from(ebpf.map("USAGE").unwrap())?;
    let mut prev_total_ns: u64 = 0;
    let mut prev_rapl = rapl_uj();
    let mut prev_t = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tick.tick() => {
                // Aggregate cumulative CPU-ns by comm, and the grand total.
                let mut by_comm: Std<String, u64> = Std::new();
                let mut total_ns: u64 = 0;
                for item in usage.iter() {
                    if let Ok((_pid, st)) = item {
                        let c = cstr(&st.comm[..COMM_LEN]);
                        *by_comm.entry(c).or_insert(0) += st.cpu_ns;
                        total_ns += st.cpu_ns;
                    }
                }
                // Work done in THIS interval (cumulative deltas).
                let dt = prev_t.elapsed().as_secs_f64().max(0.001);
                let busy_ns_delta = total_ns.saturating_sub(prev_total_ns) as f64;
                prev_total_ns = total_ns; prev_t = Instant::now();

                // System power: RAPL delta -> watts, else flat TDP model.
                let cur_rapl = rapl_uj();
                let system_w = match (prev_rapl, cur_rapl) {
                    (Some(a), Some(b)) => (b.wrapping_sub(a) as f64) / 1e6 / dt, // µJ -> J -> W
                    _ => tdp,
                };
                prev_rapl = cur_rapl;
                *sys_w.lock().unwrap() = system_w;

                // Attribute power by each comm's share of busy time this interval.
                // (Cumulative shares approximate when churn is low; see chapter.)
                let mut watts: Std<String, f64> = Std::new();
                if total_ns > 0 {
                    println!("\nsystem ~{:.2} W   (busy {:.1} ms/interval)", system_w, busy_ns_delta/1e6);
                    println!("{:<16} {:>8} {:>10}", "COMM", "SHARE%", "WATTS");
                    let mut rows: Vec<(&String,&u64)> = by_comm.iter().collect();
                    rows.sort_by(|a,b| b.1.cmp(a.1));
                    for (comm, ns) in rows.into_iter().take(12) {
                        let share = *ns as f64 / total_ns as f64;
                        let w = share * system_w;
                        watts.insert(comm.clone(), w);
                        println!("{:<16} {:>7.1}% {:>9.2}", comm, share*100.0, w);
                    }
                }
                *snap.lock().unwrap() = watts;
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
