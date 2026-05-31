---
title: "L7 tracing: HTTP"
order: 29
part: Networking
description: "Parse application-layer HTTP out of raw packets with a socket-filter program — walking Ethernet, IPv4, and TCP headers to the payload — and weigh it against the syscall/uprobe approach for encrypted or buffer-level L7."
duration: 30 minutes
---

Connection-level tools (Ch 27–28) tell you *that* a connection happened;
**L7** tracing tells you *what was said* on it — the HTTP method and
path, the response status. This chapter does it with a **socket
filter**, the first program type that inspects packet *contents*, and
then weighs it against the syscall/uprobe route for the cases a socket
filter can't reach.

The code is in `examples/29-http-l7/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

## A socket filter reads the wire

A `socket_filter` program is attached to a socket and runs on every
packet delivered to it. Point it at a raw `AF_PACKET` socket bound to an
interface and it sees *all* traffic on that interface — at the byte
level. The work is parsing down to the payload and recognizing HTTP:

{% include excalidraw.html
   file="l7-socketfilter"
   alt="A socket filter receives each packet, walks the Ethernet, IPv4, and TCP headers to find the TCP payload, checks whether it begins with an HTTP method or HTTP/ response, captures the first line, and emits it to user space via a ring buffer."
   caption="Figure 29.1 — parsing HTTP out of packets with a socket filter" %}

```rust
let ethertype = u16::from_be(ctx.load::<u16>(12)?);   // IPv4?
let verihl    = ctx.load::<u8>(ETH_HLEN)?;            // IHL == 5 (no options)
let proto     = ctx.load::<u8>(ETH_HLEN + 9)?;        // TCP?
let doff      = ctx.load::<u8>(ETH_HLEN + 20 + 12)?;  // TCP data offset
let payload   = ETH_HLEN + 20 + ((doff >> 4) as usize) * 4;
if looks_http(&ctx.load::<[u8;5]>(payload)?) { /* capture first line */ }
```

Two simplifications keep the verifier happy and the code readable: we
handle **IPv4 with no options** (IHL == 5, so the IP header is exactly
20 bytes) and we parse the **TCP data offset** to skip TCP options. Both
are flagged — real-world parsers handle options and IPv6.

Each `ctx.load::<T>(offset)` is a **bounded** read into the packet — the
socket-filter analogue of `read_at`, with the kernel checking the offset
is in range. That bounds requirement is also *why* we special-case
IPv4-without-options: a variable-length IP header would make the payload
offset unprovable to the verifier, so we assert `IHL == 5` and compute
from a fixed 20-byte header. When the first five payload bytes look like
HTTP, we copy the 4-tuple and the first line and emit:

```rust
let tcp_off = ETH_HLEN + 20;
if looks_http(&ctx.load::<[u8; 5]>(payload)?) {
    if let Some(mut slot) = EVENTS.reserve::<HttpEvent>(0) {
        let ev = slot.as_mut_ptr();
        unsafe {
            (*ev).saddr = ctx.load::<u32>(ETH_HLEN + 12)?;
            (*ev).daddr = ctx.load::<u32>(ETH_HLEN + 16)?;
            (*ev).sport = ctx.load::<u16>(tcp_off)?;
            (*ev).dport = ctx.load::<u16>(tcp_off + 2)?;
            let _ = ctx.load_bytes(payload, &mut (*ev).line);   // first ~80 bytes
        }
        slot.submit(0);
    }
}
```

## Attaching to a raw socket

Here's what makes a socket filter different from every program so far:
it's not attached to a named hook — it's attached to an actual **socket
file descriptor**. So user space first opens a raw `AF_PACKET` socket
bound to the interface, then hands its fd to `SocketFilter::attach`:

```rust
let sock = open_packet_socket(&ifname)?;   // AF_PACKET/SOCK_RAW bound to the NIC (libc)
let prog: &mut SocketFilter = ebpf.program_mut("http_filter").unwrap().try_into()?;
prog.load()?;
prog.attach(sock.as_fd())?;                // the filter now sees every frame on that socket
```

`AF_PACKET` + `SOCK_RAW` is what delivers *raw frames including the
Ethernet header* to the socket — which is why the parser starts at
offset 0 with a 14-byte `ETH_HLEN`. Binding to one interface scopes the
capture to that NIC. After that it's the familiar `RingBuf` drain:
decode each `HttpEvent`, print the flow and request line, and count by
method (`ebpf_http_lines_total{method}`).

## The complement: syscalls and uprobes

A socket filter sees the **wire**, which has one hard limit: **HTTPS is
ciphertext**. You will never parse an HTTP line out of an encrypted
packet. The complementary L7 technique traces the **syscalls** that move
the data — `sys_enter_write`/`sys_enter_read`, or `sendto`/`recvfrom` —
and inspects the *buffer*, which holds plaintext regardless of whether
TLS encrypts it on the way out. (That's the same boundary `sslsniff`
tapped in Chapter 17, just at the syscall rather than the library.)

So the choice:

- **Socket filter (this chapter)** — cleartext L7 across all connections
  on an interface, with full packet context. Blind to TLS.
- **Syscall tracepoint / uprobe** — buffer-level L7 that survives
  encryption, but you see every read/write and must attribute it to a
  connection yourself.

## Build, deploy, observe

```bash
scripts/lab/provision-vm.sh ebpf-peer    # if needed
cd examples/29-http-l7 && ./demo.sh
```

The demo runs an HTTP server on the peer and drives GET/POST from the
target while the filter watches the target's interface:

```text
FLOW                                     REQUEST / RESPONSE LINE
10.0.0.21:51122 → 10.0.0.32:8000         GET / HTTP/1.1
10.0.0.21:51124 → 10.0.0.32:8000         POST /submit HTTP/1.1
10.0.0.32:8000  → 10.0.0.21:51122        HTTP/1.0 200 OK
```

`ebpf_http_lines_total{method}` in Grafana breaks requests down by
method — the start of an L7 dashboard.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `rate(ebpf_http_lines_total[1m])` — HTTP request lines per second.

## Cross-check

```bash
[vm]$ sudo tcpdump -i any -A 'tcp port 8000' | grep -E 'GET|POST|HTTP/'
```

`tcpdump`'s view of the same cleartext should match the filter's.

## What you learned

- A **`socket_filter`** inspects packet contents; bind a raw
  `AF_PACKET` socket and attach the filter to it.
- Walk Ethernet → IPv4 → TCP (parsing the data offset) to reach the
  payload, then recognize the HTTP line.
- Socket filters are blind to **TLS**; syscall/uprobe L7 is the
  encrypted-traffic complement.

Next: **`sockops`** — reacting to TCP connection lifecycle from a
cgroup.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Risks: `SocketFilter::attach` + `SkBuffContext` `load`/`load_bytes` in
aya 0.13.x; the `AF_PACKET` socket setup; the no-IP-options (IHL==5)
simplification and TCP-data-offset math; cleartext only. The first build
and run are the test.*
