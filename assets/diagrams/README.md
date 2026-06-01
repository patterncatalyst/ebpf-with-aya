# Diagrams

Architecture and concept diagrams for the tutorial. Each diagram is a
pair:

- `<name>.svg` — the themed vector image the site embeds (this is what
  renders).
- `<name>.excalidraw` — the **editable source**. Open it at
  excalidraw.com to revise, then re-export the SVG (File → Export image
  → SVG) — or regenerate both from the spec (see below).

Embed a diagram in a chapter with the include:

```liquid
{% raw %}{% include excalidraw.html
   file="ebpf-lifecycle"
   alt="describe the diagram for screen readers"
   caption="Figure N.x — short caption" %}{% endraw %}
```

## Catalogue

| File | Chapter | Shows |
|------|---------|-------|
| `lab-topology` | 2 | host / target VM / peer VM and what runs where |
| `obs-data-path` | 3 | kernel → map → loader → OTLP → Grafana |
| `reports-in` | 3 | opensnoop: one probe, two faces of output (terminal + Grafana) |
| `workspace-build` | 4 | common / ebpf / loader crates + aya-build flow |
| `ebpf-lifecycle` | 5 | load → verify → JIT → attach, and hook types |
| `ebpf-runtime-loop` | 5 | the runtime loop: hook fires program → writes map → loader reads → Grafana |
| `ringbuf-path` | 6 | RingBuf event path, kernel → user space |
| `entry-exit` | 8 | entry/exit correlation via a HashMap (reused widely) |
| `mem-read-user-kernel` | 9 | user vs kernel memory reads (stages 9–12) |
| `tracepoint-flow` | 11 | anatomy of a trace (event → read → ship) |
| `uprobe-menu` | 13 | user-space probing surfaces (exe / lib / USDT) |
| `struct-btf` | 15 | struct-arg read + the BTF layout contract |
| `container-observe` | 16 | observing a process inside a container (cgroup, PID ns) |
| `tls-boundary` | 17 | capturing plaintext at the TLS boundary |
| `goroutine-states` | 19 | Go goroutine state machine |
| `go-vs-c-abi` | 19 | Go register ABI vs C ABI |
| `usdt-uprobe` | 20 | a USDT probe is a uprobe at a resolved offset |
| `jvm-observable` | 20 | the JVM's HotSpot USDT probe surface (GC, JIT, locks, alloc, …) |
| `runqlat-timeline` | 21 | run-queue latency across the sched tracepoints |
| `profiler-pipeline` | 23 | sampling profiler → folded → flame graph |
| `memleak-tracking` | 24 | outstanding allocations by call site |
| `bio-seq-random` | 25 | sequential vs random block I/O |
| `energy-attribution` | 26 | attributing system power to processes |
| `net-hooks` | 27 | where eBPF hooks sit along the network path |
| `tcp-handshake` | 27 | connection latency = connect() → SYN-ACK, across two kprobes |
| `tcp-states` | 28 | the TCP state machine the tracepoint reports |
| `l7-socketfilter` | 29 | a socket filter reading the HTTP request line off the wire |
| `sockops-cb` | 30 | sock_ops callbacks scoped to a cgroup |
| `tc-clsact` | 31 | clsact ingress/egress hooks and the tc verdict set |
| `xdp-path` | 32 | where XDP sits in the RX path (earliest, before sk_buff) and its verdicts |
| `xdp-capture` | 33 | filter in-kernel at XDP, ship only matching records via RingBuf |
| `xdp-lb` | 34 | XDP UDP load balancer: rewrite dest port round-robin across backends |
| `xdp-test-run` | 35 | BPF_PROG_TEST_RUN: run a program against a synthetic packet |
| `tcx-chain` | 36 | tcx: a kernel-ordered chain of bpf_link programs (vs legacy clsact) |
| `lsm-decide` | 37 | BPF LSM allow/deny: the return value decides (cgroup-scoped) |
| `signal-kill` | 38 | signal program: match an exec and bpf_send_signal(SIGKILL) |
| `pidhide` | 39 | rewrite the getdents64 buffer to splice out a /proc/<pid> entry |
| `lsm-file-protect` | 40 | LSM inode_permission: deny writes to one protected inode |
| `sudo-escalate` | 41 | rewrite sudo's read() buffer to forge sudoers (lab-only) |
| `security-sensor` | 42 | many security hooks → one SecEvent telemetry stream |
| `scx-simple` | 43 | sched_ext/struct_ops: task → BPF callbacks → DSQ → CPU |
| `scx-nest` | 44 | scx_nest: concentrate work on a warm nest of cores |
| `nginx-uprobe` | 45 | uprobes on nginx request functions, keyed by the request object |
| `three-signals` | 46 | one request → span + log + metric sharing a trace_id |
| `obi-arch` | 46 | OBI: eBPF probes → OTel signals (production picture) |
| `pg-probe` | 47 | postgres: uprobes for query latency + lock waits, keyed by backend pid |
| `pinning` | 48 | pin program/map/link to bpffs so they outlive the loader |
| `syscall-prog` | 49 | loader programs: BPF that issues bpf() itself (light skeletons) |
| `user-ringbuf` | 50 | the ring buffer that runs backwards: user space → BPF |
| `userspace-ebpf` | 51 | the same eBPF bytecode in a user-space VM (rbpf) |
| `kfuncs` | 52 | helpers vs kfuncs + the KF_ACQUIRE/KF_RELEASE discipline |
| `bpf-token` | 53 | delegating BPF into unprivileged containers (the four axes) |
| `bpf-timer` | 54 | deferred work: self-rescheduling timer + sleepable workqueue |
| `structops` | 55 | BPF implements a kernel vtable (TCP congestion control) |
| `dynptr` | 56 | a verifier-tracked handle to variable-length memory |
| `bpf-arena` | 56 | sparse shared heap where BPF builds pointer data structures |
| `bpf-iter` | 57 | the kernel calls your program once per element of a set |
| `core` | 58 | CO-RE: compile once, relocate to the target kernel via BTF |
| `bpf-lifecycle` | 59 | operating eBPF: pinning, link update, state continuity |
| `offload` | 60 | running eBPF off the host CPU (SmartNIC / DPU / FPGA / GPU) |
| `power` | 61 | attribute socket energy to workloads via sched_switch shares |
| `signal-correlation` | 62 | four backends, one trace_id (Tempo/Prometheus/Loki) |
| `capstone` | 63 | one request across both apps + the eBPF view, one trace_id |
| `bpftrace-python` | 64 | drive bpftrace from Python (NDJSON → live tool) |
| `bpftool-python` | 65 | drive bpftool from Python (JSON inventory/audit) |
| `bcc-tools` | 66 | drive the BCC tools suite from Python (resolve/run/parse) |

## Regenerating

`generate.py` is the spec-based compiler that produced these pairs
(clean themed SVG + valid Excalidraw JSON from one description). It
encodes the shared palette and layout helpers so new diagrams stay
visually consistent. The committed `.excalidraw` files remain the
editable source of record; `generate.py` is the convenience path.
