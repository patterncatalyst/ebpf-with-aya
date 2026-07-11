---
title: "A security sensor: telemetry and shielding"
order: 42
part: Security & LSM
description: "Close the security part by building the sensor layer of a runtime-security tool: several attacker-relevant hooks (execve, ptrace, setuid) feeding one structured event stream to Grafana, with a clear path from observing to shielding."
duration: 40 minutes
---

The last five chapters were single moves — confine a connection, kill a
process, hide a PID, protect a file, forge a read. A real runtime-security
tool (think Falco or Tetragon) is the *composition* of many such probes into
one **sensor**: a stream of structured security events that something
upstream reasons about, with the option to enforce. This chapter builds that
shape in miniature — three attacker-relevant hooks emitting one event stream
to Grafana — and shows where "observe" becomes "shield." It's the capstone
for the part: less a new trick than the architecture the tricks fit into.

The code is in `examples/42-secsensor/`. `./demo.sh` there builds, deploys,
and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="security-sensor"
   alt="Three kernel hooks — execve (new process), ptrace (inject or debug), and setuid (privilege change) — each emit a SecEvent (type, pid, comm) into a single RingBuf. User space reads the stream and sends it to Grafana as events by type and severity, and optionally feeds a shield that blocks via LSM. This is the sensor layer of a runtime-security tool, built in eBPF: observe many attacker-relevant operations as one stream, then optionally shield."
   caption="Figure 42.1 — Many hooks, one event stream — the sensor layer of a runtime-security tool" %}

## What a sensor watches

Attacks leave fingerprints in a small set of operations. A practical sensor
doesn't trace everything; it watches the moves that matter and turns each
into a typed event. We pick three representative ones:

- **`execve`** — every new process. The backbone of process telemetry: what
  ran, as whom, from where.
- **`ptrace`** — one process attaching to another's memory. Legitimate for
  debuggers, but also how code injection and credential theft begin; on a
  server it's almost always worth a look.
- **`setuid`** — a process changing its UID. The signature of privilege
  transition, exactly what the previous chapter forged.

Three tracepoints, one shared record type, one ring buffer. That uniformity
*is* the design: downstream code handles a single `SecEvent` stream instead
of three bespoke probes.

## How the code works

### One event type, three producers

```rust
// shared with user space
#[repr(C)]
pub struct SecEvent {
    pub etype: u32,     // 1=exec, 2=ptrace, 3=setuid
    pub pid:   u32,
    pub comm:  [u8; 16],
}

#[map] static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[inline(always)]
fn emit(etype: u32) {
    if let Some(mut slot) = EVENTS.reserve::<SecEvent>(0) {
        let ev = SecEvent {
            etype,
            pid: (bpf_get_current_pid_tgid() >> 32) as u32,
            comm: bpf_get_current_comm().unwrap_or_default(),
        };
        unsafe { *slot.as_mut_ptr() = ev; }
        slot.submit(0);
    }
}

#[tracepoint] pub fn on_exec(_:   TracePointContext) -> u32 { emit(1); 0 }
#[tracepoint] pub fn on_ptrace(_: TracePointContext) -> u32 { emit(2); 0 }
#[tracepoint] pub fn on_setuid(_: TracePointContext) -> u32 { emit(3); 0 }
```

Each handler does the minimum: stamp the event type, the PID, and the
process name, and submit. We don't even read the syscall arguments here — the
sensor's job is breadth (catch the operation) and let user space enrich and
score. Keeping the kernel side tiny keeps it cheap enough to run everywhere,
which is the whole point of a sensor.

### User space: classify, score, ship

```rust
let mut ring = RingBuf::try_from(ebpf.map_mut("EVENTS")?)?;
while let Some(item) = ring.next() {
    let ev: SecEvent = unsafe { core::ptr::read_unaligned(item.as_ptr() as *const _) };
    let (kind, severity) = match ev.etype {
        1 => ("exec",   "info"),
        2 => ("ptrace", "warning"),   // attaching to another process — notable
        3 => ("setuid", "warning"),   // privilege transition
        _ => ("other",  "info"),
    };
    println!("[{severity}] {kind} pid={} comm={}", ev.pid, cstr(&ev.comm));
    events.add(1, &[KeyValue::new("type", kind), KeyValue::new("severity", severity)]);
}
```

The user side attaches a **severity** to each type and exports
`ebpf_sec_events_total{type,severity}`, so a Grafana panel can show a calm
baseline of `exec` events with `ptrace`/`setuid` standing out — the
difference between a log and a *signal*. This is where, in a real tool, you'd
correlate (a `ptrace` from a process that just did a suspicious `exec`), rate-
limit, and alert.

### From observing to shielding

This sensor only watches. Making it **shield** is a step you already know how
to take: pair the detection with an LSM decision. The `ptrace` tracepoint
tells you an attach *happened*; the LSM hook `ptrace_access_check` (Chapter
37's pattern) lets you *deny* it. The clean architecture is exactly this
split — a broad, cheap **observe** layer for telemetry, and a narrow
**enforce** layer (LSM) for the operations you're willing to block — which is
how production tools keep enforcement decisions auditable and rare while
still seeing everything.

## Build, deploy, observe

```bash
cd examples/42-secsensor && ./demo.sh
```

The demo attaches the sensor and then exercises each event type on the
target: it runs commands (`exec`), strace-attaches to a process (`ptrace`),
and runs a small setuid transition. You'll see the classified lines stream
from the loader and three labelled series in `ebpf_sec_events_total` — the
`exec` line busy, `ptrace`/`setuid` punctuating it.

**In Grafana** (`127.0.0.1:3000` → Explore), graph `sum by (kind) (rate(ebpf_sec_events_total[1m]))` — the unified security stream, one line per event kind.

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`):

```bash
[vm]$ sudo bpftool prog show | grep tracepoint     # the three sensor programs
[vm]$ strace -p $(pgrep -n sleep) -e trace=none &   # generate a ptrace event
[vm]$ id; sudo -u nobody id                          # generate exec + uid events
```

Seeing a `ptrace` event appear the instant `strace` attaches — and the
`exec`/`setuid` lines track the commands you run — confirms the sensor is
catching the operations live.

## What you learned

- A **sensor** is the composition of many probes into one structured event
  stream — uniform `SecEvent` records from `execve`/`ptrace`/`setuid`, scored
  by severity and shipped to Grafana.
- Keeping the kernel side minimal (stamp type/pid/comm, submit) is what makes
  a sensor cheap enough to run broadly.
- **Observe vs. shield**: telemetry is broad and cheap; enforcement is narrow
  and via LSM — the architecture behind real runtime-security tools, and the
  through-line of this whole part.

That closes the **Security & LSM** part: confine, react, hide, protect,
escalate, and finally compose it all into a sensor. The next part leaves
security for a very different power — writing a CPU **scheduler** in BPF with
`sched_ext`.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
