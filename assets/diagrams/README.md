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
| `workspace-build` | 4 | common / ebpf / loader crates + aya-build flow |
| `ebpf-lifecycle` | 5 | load → verify → JIT → attach, and hook types |
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

Networking diagrams (packet path / hook points, two-VM topology, TCP
lifecycle, XDP-vs-tc) are authored alongside the Part 5 chapters.

## Regenerating

`generate.py` is the spec-based compiler that produced these pairs
(clean themed SVG + valid Excalidraw JSON from one description). It
encodes the shared palette and layout helpers so new diagrams stay
visually consistent. The committed `.excalidraw` files remain the
editable source of record; `generate.py` is the convenience path.
