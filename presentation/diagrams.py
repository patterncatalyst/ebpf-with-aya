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


# ---- 201: advanced ------------------------------------------------------------
def btf_core():
    s = Scene("btf-core", 1260, 560,
              title="CO-RE: compile once, run on many kernels",
              subtitle="BTF describes each kernel's types; relocations rewrite field offsets at load.")
    s.box(70, 170, 250, 110, "Your program", ["reads task->pid", "compiled once"], kind="rest")
    s.box(400, 150, 240, 90, "BTF (build)", ["types you compiled", "against — vmlinux"], kind="govern")
    s.box(400, 300, 240, 90, "BTF (target)", ["the running kernel's", "actual layout"], kind="platform")
    s.box(760, 220, 230, 110, "Loader relocates", ["field offsets fixed", "up at load time", "no recompile"], kind="svc")
    s.box(1050, 220, 150, 110, "Runs", ["portable", "one binary"], kind="data")
    s.arrow(320, 225, 400, 200, kind="neutral")
    s.arrow(640, 200, 760, 255, kind="govern")
    s.arrow(640, 340, 760, 290, kind="platform")
    s.arrow(990, 275, 1050, 275, kind="svc")
    s.panel(70, 420, 1130, 60)
    s.label(92, 455, "Without CO-RE a struct-offset change between kernels breaks the program; with it, the loader patches offsets.", size=13)
    s.write()


def container_uprobe():
    s = Scene("container-uprobe", 1280, 600,
              title="Uprobing a binary inside a rootless container",
              subtitle="The container hides the file; the symbols are split out. Two problems, two fixes.")
    s.box(70, 150, 250, 100, "App in container", ["rootless podman", "stripped binary"], kind="data")
    s.box(70, 320, 250, 100, "dbgsym package", [".debug (symbols)", "split via debuglink"], kind="govern")
    s.box(470, 150, 260, 100, "Extract + merge", ["eu-unstrip:", ".debug -> binary"], kind="svc")
    s.box(470, 320, 260, 100, "Bind-mount copy", ["host-visible inode", "the container runs"], kind="platform")
    s.box(880, 230, 240, 120, "uprobe attaches", ["symbol in .symtab", "with real offsets", "aya resolves it"], kind="rest")
    s.arrow(320, 200, 470, 200, kind="neutral")
    s.arrow(320, 370, 470, 370, kind="neutral")
    s.arrow(730, 210, 880, 270, kind="svc")
    s.arrow(730, 360, 880, 310, kind="platform")
    s.panel(70, 470, 1140, 90)
    s.label(92, 500, "Rootless podman keeps the container's binary out of the host namespace, so the (root) loader can't see it —", size=13)
    s.label(92, 522, "bind-mount a host copy in. And a split-debug .debug has a NOBITS .text, so merge it back with eu-unstrip", size=13)
    s.label(92, 544, "(the Postgres and nginx chapters). Then the symbol resolves with a real file offset.", size=13)
    s.write()


def lsm_decision():
    s = Scene("lsm-decision", 1220, 540,
              title="LSM programs: allow or deny, in-kernel",
              subtitle="An eBPF program at a Linux Security Module hook returns the verdict.")
    s.box(80, 200, 230, 100, "Operation", ["exec, open, kill,", "connect …"], kind="svc")
    s.box(420, 190, 240, 120, "LSM hook", ["bprm_check_security", "file_open, task_kill", "your eBPF runs here"], kind="govern")
    s.box(780, 120, 220, 90, "return 0", ["-> allow"], kind="platform")
    s.box(780, 280, 220, 90, "return -EPERM", ["-> deny"], kind="danger")
    s.arrow(310, 250, 420, 250, kind="svc")
    s.arrow(660, 220, 780, 165, kind="platform")
    s.arrow(660, 280, 780, 320, kind="danger")
    s.panel(80, 420, 920, 60)
    s.label(102, 455, "Unlike tracing, an LSM program's return value is a decision — this is enforcement, not just observation.", size=13)
    s.write()


def frontier():
    s = Scene("frontier", 1280, 600,
              title="The frontier: program types beyond tracing & networking",
              subtitle="Where eBPF stops observing and starts implementing kernel behavior.")
    s.box(70, 150, 270, 110, "struct_ops", ["implement a kernel", "vtable in BPF", "e.g. TCP congestion"], kind="rest")
    s.box(370, 150, 270, 110, "sched_ext", ["a whole CPU", "scheduler in BPF", "swap it at runtime"], kind="svc")
    s.box(670, 150, 270, 110, "kfuncs", ["call kernel functions", "the kernel exports", "typed, allow-listed"], kind="platform")
    s.box(970, 150, 240, 110, "bpf_timer", ["kernel-side timers", "callbacks without", "user space"], kind="govern")
    s.box(220, 320, 270, 110, "user_ringbuf", ["user -> kernel queue", "dynptr callbacks"], kind="data")
    s.box(520, 320, 270, 110, "BPF iterators", ["walk kernel objects", "cat a process table"], kind="platform")
    s.box(820, 320, 270, 110, "syscall progs", ["light skeletons,", "loader programs"], kind="svc")
    s.panel(70, 470, 1140, 70)
    s.label(92, 500, "Several of these are where Aya's kernel-side authoring is still emerging in 2026 — the reference", size=13)
    s.label(92, 522, "implementations are C, loaded with bpftool, with an Aya observer alongside. We flag each honestly.", size=13)
    s.write()


def aya_c_boundary():
    s = Scene("aya-c-boundary", 1240, 560,
              title="Where Aya is all-Rust — and where C still leads",
              subtitle="An honest map of the 2026 boundary.")
    s.panel(70, 140, 540, 340, fill=PALETTE["panel"])
    s.label(96, 172, "All-Rust, today", size=15, weight="bold", color=PALETTE["platform"])
    s.box(96, 200, 490, 60, "Tracing", ["tracepoint, kprobe, fentry/fexit, uprobe"], kind="platform")
    s.box(96, 275, 490, 60, "Networking", ["XDP, tc/tcx, socket ops"], kind="platform")
    s.box(96, 350, 490, 60, "Security + maps + perf", ["LSM, all map types, ring buffers, perf events"], kind="platform")
    s.panel(650, 140, 520, 340, fill=PALETTE["panel"])
    s.label(676, 172, "C reference still leads", size=15, weight="bold", color=PALETTE["govern"])
    s.box(676, 200, 470, 60, "struct_ops / sched_ext", ["kernel vtables + schedulers — C + bpftool/scx"], kind="govern")
    s.box(676, 275, 470, 60, "BPF iterators / arena", ["emerging aya authoring; C is canonical"], kind="govern")
    s.box(676, 350, 470, 60, "kfunc call relocations", ["aya-ebpf can't emit them yet — documented"], kind="govern")
    s.write()


def correlation():
    s = Scene("correlation", 1280, 560,
              title="Correlating kernel events with distributed traces",
              subtitle="The traceparent flows through the app; eBPF ties kernel spans to it.")
    s.box(70, 170, 220, 100, "Service A", ["starts a trace", "W3C traceparent"], kind="svc")
    s.box(380, 170, 220, 100, "Service B", ["propagates it", "HTTP header"], kind="svc")
    s.box(690, 170, 240, 100, "eBPF probes", ["see the syscall/query", "tag with trace id"], kind="rest")
    s.box(1010, 170, 200, 100, "Tempo", ["one trace,", "kernel + app spans"], kind="data")
    s.arrow(290, 220, 380, 220, kind="svc", label="traceparent")
    s.arrow(600, 220, 690, 220, kind="neutral")
    s.arrow(930, 220, 1010, 220, kind="data")
    s.panel(70, 360, 1140, 80)
    s.label(92, 390, "The three signals — metrics (Mimir), logs (Loki), traces (Tempo) — join on the trace id, so a slow", size=13)
    s.label(92, 412, "request in Grafana links to the exact kernel-level latency eBPF measured underneath it.", size=13)
    s.write()


def verifier_detail():
    s = Scene("verifier-detail", 1240, 560,
              title="Inside the verifier: why it accepts or rejects",
              subtitle="Abstract interpretation over every path, with a complexity budget.")
    s.box(70, 160, 240, 110, "Load request", ["bytecode +", "map fds + BTF"], kind="svc")
    s.box(400, 120, 240, 90, "Walk every path", ["track each register", "as a value range"], kind="govern")
    s.box(400, 250, 240, 90, "Prune equivalents", ["state pruning keeps", "it from exploding"], kind="govern")
    s.box(740, 160, 240, 110, "Checks", ["bounds on memory", "bounded loops", "no leaks to user"], kind="rest")
    s.box(1040, 120, 160, 80, "accept", ["-> JIT"], kind="platform")
    s.box(1040, 250, 160, 80, "reject", ["clear error"], kind="danger")
    s.arrow(310, 215, 400, 175, kind="neutral")
    s.arrow(640, 175, 740, 200, kind="govern")
    s.arrow(640, 295, 740, 240, kind="govern")
    s.arrow(980, 190, 1040, 165, kind="platform")
    s.arrow(980, 240, 1040, 280, kind="danger")
    s.panel(70, 430, 1130, 60)
    s.label(92, 465, "Big programs hit the instruction-analysis budget — which is why you split logic across tail calls or helpers.", size=13)
    s.write()


SCENES = [
    ebpf_model, aya_workspace, ebpf_maps, ebpf_hooks,
    ringbuf_stream, xdp_path, lgtm_pipeline,
    btf_core, container_uprobe, lsm_decision, frontier,
    aya_c_boundary, correlation, verifier_detail,
]
