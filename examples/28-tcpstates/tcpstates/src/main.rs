//! tcpstates — prints TCP state transitions from sock:inet_sock_set_state.
//! Exports ebpf_tcp_state_transitions_total{newstate}.
use std::net::Ipv4Addr;
use std::time::Duration;

use aya::{maps::RingBuf, programs::TracePoint, Ebpf};
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use tcpstates_common::TcpStateEvent;

fn state_name(s: u32) -> &'static str {
    match s {
        1 => "ESTABLISHED", 2 => "SYN_SENT", 3 => "SYN_RECV", 4 => "FIN_WAIT1",
        5 => "FIN_WAIT2", 6 => "TIME_WAIT", 7 => "CLOSE", 8 => "CLOSE_WAIT",
        9 => "LAST_ACK", 10 => "LISTEN", 11 => "CLOSING", 12 => "NEW_SYN_RECV",
        _ => "?",
    }
}

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
            KeyValue::new("service.name", "ebpf-tcpstates"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/tcpstates")))?;
    let tp: &mut TracePoint = ebpf.program_mut("inet_sock_set_state").unwrap().try_into()?;
    tp.load()?;
    tp.attach("sock", "inet_sock_set_state")?;
    info!("tcpstates attached to sock:inet_sock_set_state");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-tcpstates");
    let counter = meter.u64_counter("ebpf_tcp_state_transitions_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<22} {:<22} {:<13} -> {}", "SRC", "DST", "OLD", "NEW");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<TcpStateEvent>() { continue; }
                    let ev: TcpStateEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const TcpStateEvent) };
                    let src = format!("{}:{}", Ipv4Addr::from(ev.saddr), ev.sport);
                    let dst = format!("{}:{}", Ipv4Addr::from(ev.daddr), ev.dport);
                    let new = state_name(ev.newstate);
                    println!("{:<22} {:<22} {:<13} -> {}", src, dst, state_name(ev.oldstate), new);
                    counter.add(1, &[KeyValue::new("newstate", new)]);
                }
            }
        }
    }
    provider.shutdown()?;
    Ok(())
}
