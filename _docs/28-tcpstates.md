---
title: "tcpstates"
order: 28
part: Networking
description: "Trace every TCP connection through its state machine using the single stable sock:inet_sock_set_state tracepoint — the clean, offset-light counterpart to Chapter 27's kprobes, with the endpoints handed to you directly."
duration: 25 minutes
---

Chapter 27 timed one transition (connect → established) the hard way,
with kprobes and `struct sock` offsets. This chapter watches the
**entire** TCP state machine — every connection moving through
`SYN_SENT`, `ESTABLISHED`, `FIN_WAIT`, `CLOSE`, and the rest — and does
it the *easy* way, with one tracepoint that was built for exactly this.
The contrast is the lesson: when a stable tracepoint exists, prefer it
over kprobes.

The code is in `examples/28-tcpstates/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

## One tracepoint, every transition

`sock:inet_sock_set_state` fires whenever a socket changes TCP state,
and it carries everything you need in its arguments: the old and new
state, the protocol, and both endpoints' addresses and ports.

{% include excalidraw.html
   file="tcp-states"
   alt="The TCP state machine: CLOSE to SYN_SENT on connect, SYN_SENT to ESTABLISHED on SYN-ACK, ESTABLISHED to closing states, and LISTEN for passive opens — all transitions reported by the sock:inet_sock_set_state tracepoint with addresses and ports."
   caption="Figure 28.1 — the TCP state machine via one tracepoint" %}

Because the tracepoint hands you the fields, there are **no kprobes and
no `struct sock` offsets** — the soft spot from Chapter 27 simply isn't
here. You still read the tracepoint's own fields by offset (from its
format file), but those offsets are part of a stable ABI the kernel
maintains deliberately, not internal struct layout that drifts:

```rust
#[tracepoint]
pub fn inet_sock_set_state(ctx: TracePointContext) -> u32 {
    let proto = unsafe { ctx.read_at::<u8>(PROTOCOL) }.unwrap_or(0);
    if proto != IPPROTO_TCP { return 0; }                       // ignore UDP, etc.
    let ev = TcpStateEvent {
        oldstate: unsafe { ctx.read_at::<i32>(OLDSTATE) }.unwrap_or(0) as u32,
        newstate: unsafe { ctx.read_at::<i32>(NEWSTATE) }.unwrap_or(0) as u32,
        saddr:    unsafe { ctx.read_at::<[u8; 4]>(SADDR) }.unwrap_or([0; 4]),
        daddr:    unsafe { ctx.read_at::<[u8; 4]>(DADDR) }.unwrap_or([0; 4]),
        sport:    unsafe { ctx.read_at::<u16>(SPORT) }.unwrap_or(0),
        dport:    unsafe { ctx.read_at::<u16>(DPORT) }.unwrap_or(0),
    };
    if let Some(mut slot) = EVENTS.reserve::<TcpStateEvent>(0) {
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
    0
}
```

Every field comes straight from the tracepoint record via `read_at` at
the offset the format file lists — no pointer to follow, no helper
copy. The only filter is `PROTOCOL`: `inet_sock_set_state` fires for any
inet socket, so we drop non-TCP early. Reading the addresses as
`[u8; 4]` rather than a `u32` keeps the bytes in wire order, so user
space can hand them straight to `Ipv4Addr::from` without a byte swap.

## The user side

One tracepoint, one attach, then map the state numbers to names:

```rust
let tp: &mut TracePoint = ebpf.program_mut("inet_sock_set_state").unwrap().try_into()?;
tp.load()?;
tp.attach("sock", "inet_sock_set_state")?;

fn state_name(s: u32) -> &'static str {
    match s { 1=>"ESTABLISHED", 2=>"SYN_SENT", 3=>"SYN_RECV", 4=>"FIN_WAIT1",
              5=>"FIN_WAIT2", 6=>"TIME_WAIT", 7=>"CLOSE", 8=>"CLOSE_WAIT",
              9=>"LAST_ACK", 10=>"LISTEN", 11=>"CLOSING", _=>"?" }
}
// in the drain loop, per event:
let new = state_name(ev.newstate);
counter.add(1, &[KeyValue::new("newstate", new)]);
```

The TCP state constants (`TCP_ESTABLISHED = 1`, and so on) are kernel
ABI, so the name mapping is stable. Counting transitions by `newstate`
turns into a connection-health signal: a rising share of `TIME_WAIT` or
`CLOSE_WAIT` points at connection churn or sockets that never get
closed.

## Stable tracepoint vs. kprobe — when to choose which

This is the through-line of kernel tracing in one comparison:

- **kprobe** (Ch 27): attaches to *any* kernel function, so it can reach
  things no tracepoint exposes — but you depend on that function's name
  and its arguments'/structs' layout, which are internal and can change.
  Powerful, fragile.
- **tracepoint** (here): only exists where kernel developers placed one,
  but those are a **maintained ABI** with stable fields. Less reach,
  far more durable.

The rule of thumb: reach for a tracepoint when one fits; drop to a kprobe
(ideally with CO-RE) when nothing else can see what you need.

## Build, deploy, observe

```bash
scripts/lab/provision-vm.sh ebpf-peer    # if the peer isn't up yet
cd examples/28-tcpstates && ./demo.sh
```

The demo opens and closes connections from the target to the peer; you
see them walk the state machine:

```text
SRC                    DST                    OLD           -> NEW
10.0.0.21:0            10.0.0.32:8080         CLOSE         -> SYN_SENT
10.0.0.21:53344        10.0.0.32:8080         SYN_SENT      -> ESTABLISHED
10.0.0.21:53344        10.0.0.32:8080         ESTABLISHED   -> CLOSE
```

`ebpf_tcp_state_transitions_total{newstate}` in Grafana shows the mix of
states over time — a useful health signal (a spike in `TIME_WAIT` or
`CLOSE_WAIT`, say, points at connection-churn or leaked sockets).

## Cross-check

```bash
[vm]$ sudo tcpstates-bpfcc
[vm]$ cat /sys/kernel/tracing/events/sock/inet_sock_set_state/format
```

## What you learned

- `sock:inet_sock_set_state` traces **every** TCP state transition with
  the endpoints included — no kprobes, no struct-offset chasing.
- Tracepoint fields are a **stable ABI**; internal struct layout (Ch 27)
  is not — prefer a tracepoint when one fits.
- State-mix over time is a practical connection-health signal.

Next part-section: **L7 tracing** (HTTP via socket filters and syscall
tracepoints) and **`sockops`**.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Risks: the tracepoint field offsets (verify vs. the format file) and the
sport/dport byte order as this tracepoint stores them; IPv4 fields shown.
The first build and run are the test.*
