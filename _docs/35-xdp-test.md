---
title: "Testing eBPF with BPF_PROG_TEST_RUN"
order: 35
part: Networking
description: "Unit-test eBPF programs without a NIC or live traffic: feed a hand-built packet to a loaded program with BPF_PROG_TEST_RUN, get back the verdict, output buffer, and duration, and assert on them — turning a verifier-only check into a real test suite."
duration: 40 minutes
---

Everything so far has been tested by *running* it — provision a peer,
generate traffic, watch Grafana. That's integration testing, and it's
slow, flaky, and impossible in CI without a kernel and a NIC. The kernel
offers something better for the inner loop: **`BPF_PROG_TEST_RUN`**, a
`bpf()` command that runs a loaded program against an input *you* supply
and hands back the return value, any modified output buffer, and how long
it took. You build a packet as a byte array, run the program on it, and
assert the verdict — no traffic, no second VM, just a deterministic test.

The code is in `examples/35-xdp-test/`. `./demo.sh` there builds, deploys,
and runs the test binary on the target (the syscall needs privileges); its
`README.md` covers what it does.

{% include excalidraw.html
   file="xdp-test-run"
   alt="A synthetic packet built as a byte array (Ethernet + IPv4 + ICMP or TCP) is passed as data_in to BPF_PROG_TEST_RUN, which runs the loaded program. It returns the verdict (retval: XDP_DROP or XDP_PASS), an output buffer (data_out), the run duration, and any map side-effects. You assert on the verdict and map state in a test you can run anywhere with privileges."
   caption="Figure 35.1 — Run the real program against a packet you build" %}

## What BPF_PROG_TEST_RUN does

You hand the kernel a loaded program's file descriptor and a buffer of
input bytes. For an XDP or tc program the buffer is a raw packet starting
at the Ethernet header — exactly what the program would see on the wire.
The kernel wraps it in the right context, runs the program (optionally many
times, for benchmarking), and returns:

- the program's **return value** (`XDP_DROP`, `XDP_PASS`, a `TC_ACT_*`, …),
- the **output buffer**, which for a program that *rewrites* packets (like
  the Chapter 34 load balancer) contains the modified bytes,
- the **duration** per run, and
- any **map side-effects** — the program really executed, so a counter it
  bumped is now bumped.

That covers nearly everything you'd want to assert: did the program reach
the right verdict, did it mutate the packet correctly, and did it update
its maps. The one thing it doesn't exercise is the *attachment* (the hook
firing) — that still needs an integration run.

## How the code works

### The program under test

A trimmed version of the Chapter 32 filter — drop ICMP, pass everything
else — plus a per-protocol counter so we can also assert a side-effect:

```rust
#[map] static PKTS: HashMap<u32, u64> = HashMap::with_max_entries(8, 0);

#[xdp]
pub fn xdp_filter(ctx: XdpContext) -> u32 {
    try_filter(&ctx).unwrap_or(xdp_action::XDP_PASS)
}
fn try_filter(ctx: &XdpContext) -> Result<u32, ()> {
    let eth: *const EthHdr = unsafe { ptr_at(ctx, 0)? };
    if unsafe { (*eth).ether_type } != EtherType::Ipv4 { return Ok(xdp_action::XDP_PASS); }
    let ip: *const Ipv4Hdr = unsafe { ptr_at(ctx, EthHdr::LEN)? };
    let proto = unsafe { (*ip).proto };
    bump(&PKTS, proto as u32, 1);
    if proto == IpProto::Icmp { return Ok(xdp_action::XDP_DROP); }
    Ok(xdp_action::XDP_PASS)
}
```

### Building packets in user space

A test is only as good as its inputs, so the harness constructs frames
byte by byte. An IPv4 packet is an Ethernet header (14 bytes, `ether_type`
= `0x0800`) followed by a minimal IPv4 header whose protocol field decides
the rest:

```rust
fn ipv4_packet(proto: u8) -> Vec<u8> {
    let mut p = vec![0u8; 14 + 20 + 8];          // eth + ipv4 + a little payload
    p[12..14].copy_from_slice(&0x0800u16.to_be_bytes()); // ether_type = IPv4
    p[14] = 0x45;                                 // IPv4, IHL=5
    p[23] = proto;                                // protocol byte (1=ICMP, 6=TCP)
    p
}
```

`ipv4_packet(1)` is an ICMP packet, `ipv4_packet(6)` a TCP one; a frame
with a non-`0x0800` `ether_type` exercises the "not IPv4 → pass" path.

### Running and asserting

```rust
let prog: &mut Xdp = ebpf.program_mut("xdp_filter").unwrap().try_into()?;
prog.load()?;

let cases = [
    ("ICMP → DROP", ipv4_packet(1), xdp_action::XDP_DROP),
    ("TCP  → PASS", ipv4_packet(6), xdp_action::XDP_PASS),
    ("ARP  → PASS", arp_packet(),    xdp_action::XDP_PASS),
];
let mut failures = 0;
for (name, pkt, expected) in &cases {
    let retval = run_test(prog, pkt)?;           // BPF_PROG_TEST_RUN, returns the verdict
    let ok = retval == *expected;
    println!("{:<14} got={retval} want={expected}  {}", name, if ok {"PASS"} else {"FAIL"});
    if !ok { failures += 1; }
}
```

`run_test` is the example's wrapper around `BPF_PROG_TEST_RUN`. It takes the
program's file descriptor and the input bytes, issues the `bpf()` syscall
with the test sub-command, and reads back the `retval` the kernel filled
in. The loop is an ordinary test table: a name, an input, an expected
verdict — the same shape you'd write for any pure function, which is
exactly what a verdict-returning eBPF program *is* from the outside.

Because the program really ran, its map is live too. After the cases we
assert the side-effect:

```rust
let pkts: HashMap<_, u32, u64> = HashMap::try_from(ebpf.take_map("PKTS").unwrap())?;
assert!(pkts.get(&1, 0).unwrap_or(0) >= 1, "ICMP counter should have moved");
```

The process exits non-zero if any case fails, so this drops straight into
CI: build on the host, run the test binary on a kernel under `sudo`, fail
the pipeline on a bad verdict — all without sending a single real packet.

## Build, deploy, observe

```bash
cd examples/35-xdp-test && ./demo.sh
```

The demo builds the test binary, ships it to the target, and runs it under
`sudo` (the syscall needs `CAP_BPF`). You'll see a small table — each case
with its got/want verdict and PASS/FAIL — and a zero exit code when all
pass. No peer VM and no traffic generation are involved; this is the fast
inner-loop counterpart to the integration demos in the other chapters.

## Cross-check

There's no separate tool to compare against here — the test *is* the
check. But you can sanity-check the mechanism with `bpftool`, which can
itself run a program against a packet file:

```bash
[vm]$ sudo bpftool prog list                      # find the loaded prog id
[vm]$ sudo bpftool prog run id <ID> data_in pkt.bin data_out out.bin repeat 1
```

`bpftool prog run` is the same `BPF_PROG_TEST_RUN` underneath; getting the
same verdict from it as from your harness confirms you built the packet and
read the result correctly.

## What you learned

- **`BPF_PROG_TEST_RUN`** runs a loaded program against an input buffer you
  build, returning the verdict, the (possibly rewritten) output, the
  duration, and real map side-effects — eBPF unit testing without a NIC.
- Constructing **synthetic packets** byte by byte and driving a program
  through a **test table** of (input, expected verdict) pairs.
- This is the fast inner loop; attachment still needs an integration run,
  but verdict and mutation logic can be tested deterministically in CI.

Next, Chapter 36 closes the part with **`tcx`** — the modern, link-based
way to attach tc programs that supersedes the clsact wiring from
Chapter 31.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: the exact mechanism behind `run_test` —
whether the installed Aya exposes a `test_run` directly or (as the example
does) the `BPF_PROG_TEST_RUN` `bpf()` command must be issued via a syscall
wrapper, the `bpf_attr` test layout, and `prog.fd()` to obtain the program
descriptor; that the kernel accepts a 14-byte L2 header for XDP test input;
and that map side-effects are visible after the run.*
