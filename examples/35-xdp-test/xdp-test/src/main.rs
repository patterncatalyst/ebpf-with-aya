//! xdp-test — a BPF_PROG_TEST_RUN harness for xdp_filter. Builds synthetic
//! packets, runs the loaded program against each, asserts the verdict, and
//! checks a map side-effect. Exits non-zero if any case fails. Run under sudo
//! (BPF_PROG_TEST_RUN needs CAP_BPF). No NIC and no live traffic involved.
use std::os::fd::{AsFd, AsRawFd};

use aya::{maps::HashMap, programs::Xdp, Ebpf};

// xdp_action return values.
const XDP_DROP: u32 = 1;
const XDP_PASS: u32 = 2;

// bpf() command number for BPF_PROG_TEST_RUN.
const BPF_PROG_TEST_RUN: libc::c_long = 10;

// The "test" sub-struct of union bpf_attr (zero-padded; the kernel reads up to
// the size we pass). VERIFY this layout against your kernel's bpf_attr.
#[repr(C)]
#[derive(Default)]
struct TestRunAttr {
    prog_fd: u32,
    retval: u32,
    data_size_in: u32,
    data_size_out: u32,
    data_in: u64,
    data_out: u64,
    repeat: u32,
    duration: u32,
    ctx_size_in: u32,
    ctx_size_out: u32,
    ctx_in: u64,
    ctx_out: u64,
    flags: u32,
    cpu: u32,
    batch_size: u32,
    _pad: u32,
}

/// Run the program (by fd) against `pkt`, returning its verdict.
fn run_test(prog_fd: i32, pkt: &[u8]) -> anyhow::Result<u32> {
    let mut out = vec![0u8; pkt.len() + 256];
    let mut attr = TestRunAttr {
        prog_fd: prog_fd as u32,
        data_in: pkt.as_ptr() as u64,
        data_size_in: pkt.len() as u32,
        data_out: out.as_mut_ptr() as u64,
        data_size_out: out.len() as u32,
        repeat: 1,
        ..Default::default()
    };
    let rc = unsafe {
        libc::syscall(
            libc::SYS_bpf,
            BPF_PROG_TEST_RUN,
            &mut attr as *mut _ as *mut libc::c_void,
            std::mem::size_of::<TestRunAttr>() as u32,
        )
    };
    if rc < 0 {
        return Err(anyhow::anyhow!(
            "BPF_PROG_TEST_RUN failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(attr.retval)
}

fn ipv4_packet(proto: u8) -> Vec<u8> {
    let mut p = vec![0u8; 14 + 20 + 8]; // eth + minimal IPv4 + a little payload
    p[12..14].copy_from_slice(&0x0800u16.to_be_bytes()); // ether_type = IPv4
    p[14] = 0x45; // version 4, IHL 5
    p[23] = proto; // protocol byte (1=ICMP, 6=TCP)
    p
}

fn arp_packet() -> Vec<u8> {
    let mut p = vec![0u8; 14 + 28];
    p[12..14].copy_from_slice(&0x0806u16.to_be_bytes()); // ether_type = ARP
    p
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/xdp-test")))?;
    let fd = {
        let prog: &mut Xdp = ebpf.program_mut("xdp_filter").unwrap().try_into()?;
        prog.load()?;
        prog.fd()?.as_fd().as_raw_fd()
    };

    let cases: [(&str, Vec<u8>, u32); 3] = [
        ("ICMP -> DROP", ipv4_packet(1), XDP_DROP),
        ("TCP  -> PASS", ipv4_packet(6), XDP_PASS),
        ("ARP  -> PASS", arp_packet(), XDP_PASS),
    ];

    let mut failures = 0;
    println!("{:<14} {:>5} {:>5}  {}", "CASE", "got", "want", "result");
    for (name, pkt, want) in &cases {
        let got = run_test(fd, pkt)?;
        let ok = got == *want;
        println!("{:<14} {:>5} {:>5}  {}", name, got, want, if ok { "PASS" } else { "FAIL" });
        if !ok {
            failures += 1;
        }
    }

    // The program really executed, so its map moved. ICMP + TCP cases were IPv4.
    let pkts: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("PKTS").unwrap())?;
    let icmp = pkts.get(&1, 0).unwrap_or(0);
    println!("PKTS[icmp] = {icmp} (expect >= 1)");
    if icmp < 1 {
        failures += 1;
    }

    if failures == 0 {
        println!("\nall cases passed");
        Ok(())
    } else {
        Err(anyhow::anyhow!("{failures} case(s) failed"))
    }
}
