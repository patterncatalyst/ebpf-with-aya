# Example 29 — HTTP L7 tracing (socket filter)

Parse application-layer **HTTP** out of raw packets with a **socket
filter** — the first program type that inspects packet *contents*.

## What this shows (new)

- A **`socket_filter`** program (new type): runs on every packet
  delivered to the socket it's attached to.
- **Walking the headers** Ethernet → IPv4 → TCP to find the payload
  offset (parsing the TCP data-offset to skip options).
- Detecting an **HTTP request/response line** (`GET `, `POST`, `HTTP/`…)
  and capturing it with the 4-tuple.
- User space opens a raw **`AF_PACKET`** socket bound to an interface and
  attaches the filter to it.

## The complementary approach (syscall tracepoints)

A socket filter sees the **wire**, so it can't read HTTPS (that's
ciphertext — see Ch 17's `sslsniff` for the uprobe-at-the-TLS-boundary
answer). The other L7 technique is to trace **`sys_enter_write` /
`sys_enter_read`** (or `sendto`/`recvfrom`) and inspect the *buffer*,
which catches HTTP after TLS has decrypted it and regardless of packet
layout — at the cost of seeing every read/write. Pick the wire (this
example) for cleartext L7 across many connections; pick syscalls/uprobes
for encrypted or buffer-level L7.

## Run it (two-VM)

```bash
scripts/lab/provision-vm.sh ebpf-peer    # if needed
./demo.sh
```

```
FLOW                                     REQUEST / RESPONSE LINE
10.0.0.21:51122 → 10.0.0.32:8000         GET / HTTP/1.1
10.0.0.21:51124 → 10.0.0.32:8000         POST /submit HTTP/1.1
10.0.0.32:8000  → 10.0.0.21:51122        HTTP/1.0 200 OK
```

`ebpf_http_lines_total{method}` in Grafana.

## ⚠ Verification status

**Unverified.** Risks: `SocketFilter::attach` taking the raw socket fd
and the `SkBuffContext` `load` / `load_bytes` API in aya 0.14.x; the
AF_PACKET socket setup (`libc`); the **no-IP-options (IHL==5)**
simplification and the TCP-data-offset math; cleartext only (HTTPS is
ciphertext on the wire). Record results in
`_plans/reconciliation-plan.md`.
