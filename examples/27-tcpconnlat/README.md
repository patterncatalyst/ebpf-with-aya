# Example 27 — tcpconnlat (TCP connection latency)

Measure how long active TCP connections take to establish — `connect()`
until the SYN-ACK comes back. This is the first **two-VM** chapter and
the first to probe the **kernel TCP stack**.

## What this shows (new)

- **kprobes on kernel TCP functions**: `tcp_v4_connect` (SYN sent) and
  `tcp_rcv_state_process` (SYN-ACK processed).
- **Keying by the `struct sock *` pointer** to correlate the two probes
  for one connection — the kernel-side analogue of the pid_tgid key.
- **Reading kernel struct fields by offset** (`skc_daddr`, `skc_dport`
  at the head of `sock_common`) with `bpf_probe_read_kernel` — and an
  candid forward-reference to **CO-RE (Ch 56)** for making those offsets
  portable.

## Two-VM lab

This needs the peer guest:

```bash
scripts/lab/provision-vm.sh ebpf-peer      # one-time
```

`tcpconnlat` runs on **ebpf-target**; the demo drives connects from the
target to a listener on **ebpf-peer**.

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # start peer listener, drive connects, run tcpconnlat
```

```
PID      COMM             DEST                   LAT(ms)
4821     curl             10.0.0.32:8080         0.412
4830     curl             10.0.0.32:8080         0.398
```

`tcp_connect_latency_ms` in Grafana — a connection-latency histogram you
can watch jump under network stress.

## Cross-check (on the target VM)

```bash
[vm]$ sudo tcpconnlat-bpfcc
[vm]$ sudo pahole -C sock_common /sys/kernel/btf/vmlinux | grep -E 'skc_daddr|skc_dport'   # confirm offsets
```

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM (two-VM setup, tcpconnlat on ebpf-target driving connects to a
listener on ebpf-peer): builds, loads, attaches both kprobes, and runs as
described. The `sock_common` field offsets (`skc_daddr`@0, `skc_dport`@12)
and the `KProbe::attach` targets (`tcp_v4_connect`,
`tcp_rcv_state_process`) can be kernel-version-specific — confirmed on
this kernel; verify with `pahole` on others, and CO-RE (Ch 56) removes the
guesswork. IPv4 only (add `tcp_v6_connect` for v6).
