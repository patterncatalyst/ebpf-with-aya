//! profile — a sampling CPU profiler. Attaches a perf_event program at 99 Hz on
//! every CPU, samples stacks for a duration, then prints FOLDED stacks
//! (flame-graph input: `comm;frame;frame;... count`). Kernel frames are
//! symbolized via /proc/kallsyms; user frames are printed as hex (point a
//! symbolizer like blazesym at them, or feed the fold to a flame-graph tool).
//!
//! Usage: profile [DURATION_SECS]   (default 10)
use std::collections::BTreeMap;
use std::time::Duration;

use aya::{
    maps::{HashMap as BpfHashMap, StackTraceMap},
    programs::{perf_event::{PerfEventScope, PerfTypeId, SamplePolicy}, PerfEvent},
    util::{kernel_symbols, online_cpus},
    Ebpf,
};
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use profile_common::{StackKey, COMM_LEN};

// perf_event_attr: software event PERF_COUNT_SW_CPU_CLOCK has config value 0.
const PERF_COUNT_SW_CPU_CLOCK: u64 = 0;

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
            KeyValue::new("service.name", "ebpf-profile"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn cstr(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}

// Greatest kernel symbol whose address is <= ip.
fn ksym(ksyms: &BTreeMap<u64, String>, ip: u64) -> String {
    match ksyms.range(..=ip).next_back() {
        Some((_, name)) => format!("{name}_[k]"),
        None => format!("{ip:#x}_[k]"),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10);

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/profile")))?;

    // Attach the perf_event program at 99 Hz on every online CPU.
    let prog: &mut PerfEvent = ebpf.program_mut("profile_cpu").unwrap().try_into()?;
    prog.load()?;
    let cpus = online_cpus().map_err(|(_, e)| anyhow::anyhow!("online_cpus: {e}"))?;
    for cpu in cpus {
        prog.attach(
            PerfTypeId::Software,
            PERF_COUNT_SW_CPU_CLOCK,
            PerfEventScope::AllProcessesOneCpu { cpu },
            SamplePolicy::Frequency(99),
            true,
        )?;
    }
    info!("profiling at 99 Hz on all CPUs for {secs}s...");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-profile");
    let samples = meter.u64_counter("ebpf_profile_samples_total").build();

    // Let it sample.
    tokio::select! {
        _ = tokio::signal::ctrl_c() => { info!("stopped early"); }
        _ = tokio::time::sleep(Duration::from_secs(secs)) => {}
    }

    // Symbolize + fold.
    let ksyms = kernel_symbols().unwrap_or_default();
    let stacks = StackTraceMap::try_from(ebpf.map("STACKS").unwrap())?;
    let counts: BpfHashMap<_, StackKey, u64> = BpfHashMap::try_from(ebpf.map("COUNTS").unwrap())?;

    let mut total = 0u64;
    for entry in counts.iter() {
        let (key, count) = match entry { Ok(kv) => kv, Err(_) => continue };
        total += count;

        // Folded line: comm;<user frames, leaf..root>;<kernel frames> count
        let mut frames: Vec<String> = vec![cstr(&key.comm[..COMM_LEN])];

        if key.ustack >= 0 {
            if let Ok(trace) = stacks.get(&(key.ustack as u32), 0) {
                for frame in trace.frames() {
                    // No user symbolizer wired in; show the address.
                    frames.push(format!("{:#x}", frame.ip));
                }
            }
        }
        if key.kstack >= 0 {
            if let Ok(trace) = stacks.get(&(key.kstack as u32), 0) {
                for frame in trace.frames() {
                    frames.push(ksym(&ksyms, frame.ip));
                }
            }
        }
        println!("{} {}", frames.join(";"), count);
    }

    samples.add(total, &[KeyValue::new("program", "profile")]);
    eprintln!("# {total} samples folded — pipe stdout to flamegraph.pl, or push to Pyroscope");
    provider.shutdown()?;
    Ok(())
}
