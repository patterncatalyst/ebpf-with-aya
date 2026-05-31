//! nginx-probe — attach uprobes to a (containerized) nginx and report a
//! per-request latency histogram. argv[1] = path to the nginx binary
//! (e.g. /proc/<worker-pid>/root/usr/sbin/nginx), argv[2] = worker pid.
//! Exports ebpf_nginx_request_latency_us{le} and ebpf_nginx_requests_total.
use std::collections::HashMap as Std;
use std::time::Duration;

use aya::{maps::HashMap, programs::UProbe, Ebpf};
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
            KeyValue::new("service.name", "ebpf-nginx-probe"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn render(hist: &Std<u32, u64>) {
    let max = hist.values().copied().max().unwrap_or(1).max(1);
    println!("\n  latency (us)        count");
    let mut slots: Vec<u32> = hist.keys().copied().collect();
    slots.sort();
    for s in slots {
        let c = hist[&s];
        let lo = 1u64 << s;
        let bar = "█".repeat(((c as f64 / max as f64) * 40.0) as usize);
        println!("  [{lo:>8}, {:>8})  {c:>8} {bar}", lo << 1);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let target = std::env::args().nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: nginx-probe <nginx-binary-path> [worker-pid]"))?;
    let pid: Option<i32> = std::env::args().nth(2).and_then(|s| s.parse().ok());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/nginx-probe")))?;
    for (prog_name, fn_name) in [
        ("req_start", "ngx_http_process_request"),
        ("req_done", "ngx_http_finalize_request"),
    ] {
        let p: &mut UProbe = ebpf.program_mut(prog_name).unwrap().try_into()?;
        p.load()?;
        p.attach(Some(fn_name), 0, &target, pid)?;
    }
    info!("attached uprobes to {target} (pid {pid:?}) — measuring request latency");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-nginx-probe");
    let lat: Counter<u64> = meter.u64_counter("ebpf_nginx_request_latency_us").build();
    let total: Counter<u64> = meter.u64_counter("ebpf_nginx_requests_total").build();
    let hmap: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("HIST").unwrap())?;
    let mut last: Std<u32, u64> = Std::new();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("detaching"); break; }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                let mut cur: Std<u32, u64> = Std::new();
                for res in hmap.iter() { let (s, c) = res?; cur.insert(s, c); }
                for (s, c) in &cur {
                    let prev = last.get(s).copied().unwrap_or(0);
                    if *c > prev {
                        let d = c - prev;
                        let le = (1u64 << (s + 1)).to_string();
                        lat.add(d, &[KeyValue::new("le", le)]);
                        total.add(d, &[]);
                    }
                }
                last = cur.clone();
                render(&cur);
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
