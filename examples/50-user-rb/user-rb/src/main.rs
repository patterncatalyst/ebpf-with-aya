//! user-rb — PRODUCER side. Submit a stream of samples into the user ring
//! buffer, trigger the consumer to drain (by calling getpid), and read back
//! the aggregate the callback built. Exports ebpf_userrb_messages_total + sum.
//!
//! EXPERIMENTAL: Aya's user-space UserRingBuf producer API is still settling;
//! the reserve/submit calls here are the expected shape. See README.
use std::{
    os::fd::{AsFd, AsRawFd},
    ptr,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
    time::Duration,
};

use aya::{
    maps::{HashMap, Map, MapData},
    programs::TracePoint,
    Ebpf,
};
use log::info;

// ---------------------------------------------------------------------------
// Minimal user-ring-buffer PRODUCER.
//
// aya 0.13 has no high-level producer for BPF_MAP_TYPE_USER_RINGBUF (the map
// loads as `Map::Unsupported`), so we drive it directly: mmap the consumer page
// (read-only) and the producer+data pages (read-write), then reserve/submit
// using the kernel's ring-buffer record layout, exactly as libbpf does. Shape
// preserved; still experimental.
// ---------------------------------------------------------------------------
const BPF_RINGBUF_HDR_SZ: usize = 8;
const BPF_RINGBUF_BUSY_BIT: u32 = 1 << 31;

struct UserRingBuf {
    consumer: *mut u8,
    producer: *mut u8,
    data: *mut u8,
    mask: u32,
    consumer_len: usize,
    producer_len: usize,
}

struct RbEntry<'a> {
    hdr: *mut u32,
    sample: &'a mut [u8],
}

impl UserRingBuf {
    fn new(data: &MapData) -> anyhow::Result<Self> {
        let fd = data.fd().as_fd().as_raw_fd();
        let max_entries = data.info()?.max_entries();
        anyhow::ensure!(max_entries.is_power_of_two(), "ringbuf size must be a power of two");
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;

        // Consumer position page: written by the kernel, mapped read-only at offset 0.
        let consumer_len = page_size;
        let consumer = unsafe {
            libc::mmap(ptr::null_mut(), consumer_len, libc::PROT_READ, libc::MAP_SHARED, fd, 0)
        };
        anyhow::ensure!(consumer != libc::MAP_FAILED, "mmap consumer page failed");

        // Producer page + data pages (mapped twice by the kernel), read-write at offset page_size.
        let producer_len = page_size + 2 * max_entries as usize;
        let producer = unsafe {
            libc::mmap(
                ptr::null_mut(),
                producer_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                page_size as libc::off_t,
            )
        };
        anyhow::ensure!(producer != libc::MAP_FAILED, "mmap producer pages failed");

        Ok(Self {
            consumer: consumer as *mut u8,
            producer: producer as *mut u8,
            data: unsafe { (producer as *mut u8).add(page_size) },
            mask: max_entries - 1,
            consumer_len,
            producer_len,
        })
    }

    fn reserve(&mut self, size: usize) -> anyhow::Result<RbEntry<'_>> {
        let total = (size + BPF_RINGBUF_HDR_SZ).next_multiple_of(8);
        let cons_pos =
            unsafe { (*(self.consumer as *const AtomicU64)).load(Ordering::Acquire) };
        // We are the sole producer, so a plain read of our own position is fine.
        let prod_atomic = unsafe { &*(self.producer as *const AtomicU64) };
        let prod_pos = prod_atomic.load(Ordering::Acquire);

        let avail = (self.mask as u64 + 1).saturating_sub(prod_pos.wrapping_sub(cons_pos));
        anyhow::ensure!(total as u64 <= avail, "user ring buffer full");

        let offset = (prod_pos & self.mask as u64) as usize;
        let hdr = unsafe { self.data.add(offset) as *mut u32 };
        unsafe {
            hdr.write(size as u32 | BPF_RINGBUF_BUSY_BIT); // len + busy bit
            hdr.add(1).write(0); // pad
        }
        // Advance the producer position so the record is claimed.
        prod_atomic.store(prod_pos + total as u64, Ordering::Release);

        let sample = unsafe { (hdr as *mut u8).add(BPF_RINGBUF_HDR_SZ) };
        Ok(RbEntry { hdr, sample: unsafe { std::slice::from_raw_parts_mut(sample, size) } })
    }
}

impl Drop for UserRingBuf {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.consumer as *mut _, self.consumer_len);
            libc::munmap(self.producer as *mut _, self.producer_len);
        }
    }
}

impl RbEntry<'_> {
    fn copy_from_slice(&mut self, src: &[u8]) {
        self.sample.copy_from_slice(src);
    }

    fn submit(self) -> anyhow::Result<()> {
        // Clear the busy bit to publish the record to the kernel consumer.
        let hdr = unsafe { &*(self.hdr as *const AtomicU32) };
        let len = hdr.load(Ordering::Relaxed) & !BPF_RINGBUF_BUSY_BIT;
        hdr.store(len, Ordering::Release);
        Ok(())
    }
}

impl TryFrom<&mut Map> for UserRingBuf {
    type Error = anyhow::Error;

    fn try_from(map: &mut Map) -> Result<Self, Self::Error> {
        match map {
            Map::Unsupported(data) => UserRingBuf::new(data),
            _ => anyhow::bail!("USER_RB is not a user ring buffer map"),
        }
    }
}
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
    // aya has no typed USER_RINGBUF map, so load it as an "unsupported" map and
    // drive it ourselves via MapData::fd() (Map::Unsupported below).
    let mut ebpf = aya::EbpfLoader::new()
        .allow_unsupported_maps()
        .load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/user-rb")))?;
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
