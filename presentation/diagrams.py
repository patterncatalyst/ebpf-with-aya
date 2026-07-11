"""Diagrams for the eBPF-with-Aya decks (101 + 201). One source of truth;
each Scene emits SVG + .excalidraw + PNG for slide embedding."""
from dgen import Scene, PALETTE


# ---- 101: the core eBPF model --------------------------------------------------
def ebpf_model():
    s = Scene("ebpf-model", 1240, 600,
              title="How an eBPF program gets into the kernel",
              subtitle="Write in Rust, compile to BPF bytecode, load → verify → attach.")
    s.box(70, 150, 250, 110, "Your Rust code", ["#[tracepoint] fn …", "compiled to BPF bytecode"], kind="svc")
    s.box(400, 150, 240, 110, "Loader (user space)", ["aya: Ebpf::load()", "runs under CAP_BPF"], kind="svc")
    s.box(720, 130, 210, 90, "Verifier", ["proves safety:", "bounds, loops, memory"], kind="govern")
    s.box(720, 250, 210, 90, "JIT", ["bytecode → native", "runs at kernel speed"], kind="platform")
    s.box(1000, 150, 180, 110, "Attached hook", ["fires on the event", "(syscall, packet, …)"], kind="rest")
    s.arrow(320, 205, 400, 205, kind="neutral")
    s.arrow(640, 195, 720, 175, kind="govern", label="load")
    s.arrow(825, 220, 825, 250, kind="platform")
    s.arrow(930, 250, 1000, 210, kind="rest", label="attach")
    s.panel(400, 380, 530, 70)
    s.label(420, 410, "The verifier is the trust boundary: reject anything unsafe, at load time.", size=13)
    s.write()


def aya_workspace():
    s = Scene("aya-workspace", 1240, 560,
              title="The Aya workspace: two halves that share types",
              subtitle="One Cargo workspace, two targets — kernel BPF and user-space host.")
    s.box(90, 160, 300, 130, "hello-ebpf", ["#![no_std] kernel crate", "nightly + BPF target", "the program the kernel runs"], kind="rest")
    s.box(850, 160, 300, 130, "hello (loader)", ["std user-space crate", "stable Rust toolchain", "loads, attaches, reports"], kind="svc")
    s.box(470, 190, 300, 90, "hello-common", ["#[repr(C)] shared structs", "compiled into BOTH sides"], kind="data")
    s.arrow(470, 235, 390, 225, kind="data")
    s.arrow(770, 235, 850, 225, kind="data")
    s.panel(90, 360, 1060, 90)
    s.label(112, 388, "build.rs (aya-build) compiles the -ebpf crate for the BPF target and embeds the object;", size=13)
    s.label(112, 412, "bpf-linker links the bytecode. The loader includes it with include_bytes_aligned!.", size=13)
    s.write()


def ebpf_maps():
    s = Scene("ebpf-maps", 1200, 540,
              title="Maps: the shared memory between kernel and user space",
              subtitle="The program writes; the loader reads. No syscalls per event.")
    s.box(80, 170, 260, 110, "eBPF program", ["runs in kernel", "EVENTS.output(&e)"], kind="rest")
    s.box(470, 160, 240, 130, "BPF map", ["HashMap / Array /", "RingBuf / PerCpu", "lives in the kernel"], kind="data")
    s.box(850, 170, 260, 110, "Loader", ["user space", "map.read() / .next()"], kind="svc")
    s.arrow(340, 220, 470, 220, kind="rest", label="write")
    s.arrow(850, 250, 710, 250, kind="svc", label="read", label_offset=14)
    s.panel(80, 380, 1030, 80)
    s.label(102, 408, "Map choice is a design decision: per-CPU counters for cheap aggregation,", size=13)
    s.label(102, 430, "ring buffers for ordered event streams, arrays for config the loader sets.", size=13)
    s.write()


def ebpf_hooks():
    s = Scene("ebpf-hooks", 1260, 600,
              title="Where eBPF attaches: one technology, many hook points",
              subtitle="The same load→verify→attach model, aimed at different kernel surfaces.")
    s.box(520, 120, 220, 80, "eBPF program", ["verified + JITed"], kind="rest")
    s.box(70, 300, 250, 110, "Tracing", ["tracepoints, kprobes,", "fentry/fexit, uprobes", "→ observe execution"], kind="platform")
    s.box(360, 300, 250, 110, "Networking", ["XDP, tc/tcx,", "socket ops", "→ filter / redirect"], kind="svc")
    s.box(650, 300, 250, 110, "Security (LSM)", ["bprm_check, file_open,", "task_kill", "→ allow / deny"], kind="govern")
    s.box(940, 300, 250, 110, "Perf & sched", ["perf events, sched_ext,", "profiling", "→ measure / schedule"], kind="data")
    s.arrow(560, 200, 195, 300, kind="neutral")
    s.arrow(600, 200, 485, 300, kind="neutral")
    s.arrow(660, 200, 775, 300, kind="neutral")
    s.arrow(700, 200, 1065, 300, kind="neutral")
    s.panel(70, 470, 1120, 60)
    s.label(92, 505, "Learn the model once; every chapter is a different hook and a different map — the shape is the same.", size=13)
    s.write()


def ringbuf_stream():
    s = Scene("ringbuf-stream", 1200, 520,
              title="Ring buffer: streaming events to user space in order",
              subtitle="Reserve → fill → submit in the kernel; drain in a poll loop.")
    s.box(80, 180, 240, 100, "eBPF program", ["reserve::<Event>()", "write fields, submit()"], kind="rest")
    s.box(450, 170, 260, 120, "RingBuf", ["lock-free queue", "kernel → user", "back-pressure aware"], kind="data")
    s.box(840, 180, 260, 100, "Loader poll loop", ["ring.next()", "decode + export OTLP"], kind="svc")
    s.arrow(320, 230, 450, 230, kind="rest", label="submit")
    s.arrow(710, 230, 840, 230, kind="data", label="drain")
    s.panel(80, 380, 1020, 70)
    s.label(102, 410, "Ordered, allocation-free, and cheap: the modern replacement for perf buffers for event streams.", size=13)
    s.write()


def xdp_path():
    s = Scene("xdp-path", 1240, 540,
              title="XDP: a decision at the earliest possible point",
              subtitle="Runs in the driver, before an skb is even allocated.")
    s.box(70, 190, 180, 90, "NIC", ["packet arrives"], kind="platform")
    s.box(330, 190, 210, 90, "XDP program", ["inspect headers", "return an action"], kind="rest")
    s.box(640, 110, 200, 80, "XDP_PASS", ["→ up the stack"], kind="svc")
    s.box(640, 230, 200, 80, "XDP_DROP", ["→ gone, no cost"], kind="danger")
    s.box(950, 110, 210, 80, "Network stack", ["sockets, apps"], kind="svc")
    s.arrow(250, 235, 330, 235, kind="platform")
    s.arrow(540, 215, 640, 160, kind="svc")
    s.arrow(540, 255, 640, 270, kind="danger")
    s.arrow(840, 150, 950, 150, kind="svc")
    s.panel(70, 400, 1090, 70)
    s.label(92, 430, "Dropping a flood at XDP costs a few instructions per packet — the packet never touches the stack.", size=13)
    s.write()


def lgtm_pipeline():
    s = Scene("lgtm-pipeline", 1260, 540,
              title="From kernel event to Grafana panel",
              subtitle="The loader is also an OTLP exporter; the LGTM stack does the rest.")
    s.box(70, 180, 220, 100, "eBPF + loader", ["events / metrics", "on the VM"], kind="rest")
    s.box(380, 180, 200, 100, "OTLP export", ["OTLP/HTTP :4318", "from the loader"], kind="svc")
    s.box(670, 150, 230, 160, "LGTM stack", ["Loki · logs", "Grafana · dashboards", "Tempo · traces", "Mimir/Prom · metrics"], kind="data")
    s.box(990, 180, 200, 100, "Grafana", ["one pane:", "kernel truth, graphed"], kind="platform")
    s.arrow(290, 230, 380, 230, kind="rest")
    s.arrow(580, 230, 670, 230, kind="svc", label="OTLP")
    s.arrow(900, 230, 990, 230, kind="data")
    s.panel(70, 400, 1120, 60)
    s.label(92, 435, "No agent in the target, no code change in the app: the signal comes from the kernel, uniformly.", size=13)
    s.write()


SCENES = [
    ebpf_model, aya_workspace, ebpf_maps, ebpf_hooks,
    ringbuf_stream, xdp_path, lgtm_pipeline,
]
