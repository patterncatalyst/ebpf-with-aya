//! httpl7 — attaches the HTTP socket filter to a raw AF_PACKET socket bound to
//! an interface (argv[1], default "eth0") and prints captured HTTP request /
//! response lines. Exports ebpf_http_lines_total{method}.
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, BorrowedFd};
use std::time::Duration;

use aya::{maps::RingBuf, programs::SocketFilter, Ebpf};
use log::{info, warn};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use httpl7_common::{HttpEvent, LINE_CAP};

// Open an AF_PACKET raw socket bound to `ifname`, return its fd.
fn open_packet_socket(ifname: &str) -> anyhow::Result<std::os::fd::OwnedFd> {
    use std::os::fd::FromRawFd;
    const ETH_P_ALL: u16 = 0x0003;
    let fd = unsafe { libc::socket(libc::AF_PACKET, libc::SOCK_RAW, (ETH_P_ALL as u16).to_be() as i32) };
    if fd < 0 { anyhow::bail!("socket(AF_PACKET): {}", std::io::Error::last_os_error()); }
    let owned = unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) };
    let cname = std::ffi::CString::new(ifname)?;
    let ifindex = unsafe { libc::if_nametoindex(cname.as_ptr()) };
    if ifindex == 0 { anyhow::bail!("if_nametoindex({ifname}) failed"); }
    let mut sll: libc::sockaddr_ll = unsafe { std::mem::zeroed() };
    sll.sll_family = libc::AF_PACKET as u16;
    sll.sll_protocol = (ETH_P_ALL as u16).to_be();
    sll.sll_ifindex = ifindex as i32;
    let rc = unsafe {
        libc::bind(fd, &sll as *const _ as *const libc::sockaddr,
                   std::mem::size_of::<libc::sockaddr_ll>() as u32)
    };
    if rc < 0 { anyhow::bail!("bind: {}", std::io::Error::last_os_error()); }
    Ok(owned)
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
            KeyValue::new("service.name", "ebpf-httpl7"),
            KeyValue::new("service.namespace", "ebpf-with-aya"),
        ])).build();
    global::set_meter_provider(provider.clone());
    Ok(provider)
}

fn first_line(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == b'\r' || c == b'\n' || c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).into_owned()
}
fn method_of(line: &str) -> String {
    line.split(' ').next().unwrap_or("?").to_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let ifname = std::env::args().nth(1).unwrap_or_else(|| "eth0".to_string());

    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/httpl7")))?;
    let sock = open_packet_socket(&ifname)?;
    let prog: &mut SocketFilter = ebpf.program_mut("http_filter").unwrap().try_into()?;
    prog.load()?;
    let bfd: BorrowedFd = unsafe { BorrowedFd::borrow_raw(sock.as_raw_fd()) };
    prog.attach(bfd)?;
    info!("HTTP L7 filter attached to {ifname}");

    let provider = init_otel()?;
    let meter = global::meter("ebpf-httpl7");
    let counter = meter.u64_counter("ebpf_http_lines_total").build();
    let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS").unwrap())?;

    println!("{:<40} {}", "FLOW", "REQUEST / RESPONSE LINE");
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { info!("shutting down"); break; }
            _ = tokio::time::sleep(Duration::from_millis(200)) => {
                while let Some(item) = ring.next() {
                    let bytes: &[u8] = &item;
                    if bytes.len() < core::mem::size_of::<HttpEvent>() { continue; }
                    let ev: HttpEvent = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const HttpEvent) };
                    let flow = format!("{}:{} → {}:{}",
                        Ipv4Addr::from(u32::from_be(ev.saddr)), u16::from_be(ev.sport),
                        Ipv4Addr::from(u32::from_be(ev.daddr)), u16::from_be(ev.dport));
                    let line = first_line(&ev.line[..LINE_CAP]);
                    println!("{:<40} {}", flow, line);
                    counter.add(1, &[KeyValue::new("method", method_of(&line))]);
                }
            }
        }
    }
    drop(sock);
    provider.shutdown()?;
    Ok(())
}
