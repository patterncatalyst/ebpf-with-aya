//! power — read per-command on-CPU shares from the kernel and, where RAPL is
//! available, multiply by package energy to estimate per-workload watts.
//! In a VM (no RAPL) it reports the CPU-time shares instead.
use std::collections::HashMap as Map;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use aya::{maps::HashMap as BpfHashMap, programs::TracePoint, Ebpf};
use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use power_common::Comm;

const RAPL: &str = "/sys/class/powercap/intel-rapl:0/energy_uj";

fn read_rapl_uj() -> Option<u64> {
    std::fs::read_to_string(RAPL).ok()?.trim().parse().ok()
}
fn comm_str(c: &Comm) -> String {
    let end = c.name.iter().position(|&b| b == 0).unwrap_or(c.name.len());
    String::from_utf8_lossy(&c.name[..end]).into_owned()
}

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
            KeyValue::new("service.name", "ebpf-power"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let have_rapl = std::path::Path::new(RAPL).exists();
    println!("RAPL: {}", if have_rapl { "present — estimating watts" } else { "absent (VM) — reporting CPU-time shares only" });

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/power")))?;
    let prog: &mut TracePoint = ebpf.program_mut("on_switch").unwrap().try_into()?;
    prog.load()?;
    prog.attach("sched", "sched_switch")?;

    let provider = init_otel()?;
    let meter = global::meter("ebpf-power");
    let oncpu: Counter<f64> = meter.f64_counter("ebpf_oncpu_seconds_total").build();
    // estimated watts per command, surfaced via an observable gauge from a snapshot
    let watts_snap: Arc<Mutex<Vec<(String, f64)>>> = Arc::new(Mutex::new(Vec::new()));
    let snap2 = watts_snap.clone();
    let _g = meter.f64_observable_gauge("ebpf_estimated_watts")
        .with_callback(move |obs| {
            for (comm, w) in snap2.lock().unwrap().iter() {
                obs.observe(*w, &[KeyValue::new("comm", comm.clone())]);
            }
        }).build();

    let mut prev: Map<String, u64> = Map::new();
    let mut last_uj = read_rapl_uj();
    for _ in 0..15 {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // per-command on-CPU ns this interval
        let m: BpfHashMap<_, Comm, u64> = BpfHashMap::try_from(ebpf.map("ONCPU").unwrap())?;
        let mut totals: Map<String, u64> = Map::new();
        for item in m.iter() {
            let (k, v) = item?;
            *totals.entry(comm_str(&k)).or_insert(0) += v;
        }
        let mut interval: Map<String, u64> = Map::new();
        let mut total_ns: u64 = 0;
        for (c, t) in &totals {
            let d = t.saturating_sub(*prev.get(c).unwrap_or(&0));
            if d > 0 { interval.insert(c.clone(), d); total_ns += d; }
        }
        prev = totals;

        // package watts this interval (if RAPL)
        let pkg_w = if have_rapl {
            let now_uj = read_rapl_uj();
            let w = match (now_uj, last_uj) {
                (Some(n), Some(l)) if n >= l => (n - l) as f64 / 1_000_000.0, // µJ→J over 1s = W
                _ => 0.0,
            };
            last_uj = now_uj;
            w
        } else { 0.0 };

        // attribute + export, and print the top few
        let mut rows: Vec<(String, u64, f64)> = interval.iter().map(|(c, &ns)| {
            let share = if total_ns > 0 { ns as f64 / total_ns as f64 } else { 0.0 };
            (c.clone(), ns, pkg_w * share)
        }).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));

        let mut snap = Vec::new();
        for (c, ns, w) in rows.iter().take(8) {
            oncpu.add(*ns as f64 / 1e9, &[KeyValue::new("comm", c.clone())]);
            if have_rapl { snap.push((c.clone(), *w)); }
            if have_rapl { println!("{:<16} {:>6.2} W  ({:.1} ms on-CPU)", c, w, *ns as f64 / 1e6); }
            else { println!("{:<16} {:.1} ms on-CPU", c, *ns as f64 / 1e6); }
        }
        *watts_snap.lock().unwrap() = snap;
        println!("---");
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    provider.shutdown()?;
    Ok(())
}
