// deck-101.js — "eBPF with Aya · 101" (~1.5h, Rust devs new to eBPF).
// Build: export NODE_PATH=$(npm root -g) && node deck-101.js
"use strict";

const H = require("./deck-helpers.js");
const {
  COLOR, FONT, W, ASSETS,
  newDeck, addFooter, addContentTitle, addBullets, addTwoColBullets,
  addStatusTable, addCaption, addCodeSlide, addDiagramSlide, addSectionDivider, addNotes,
} = H;

const OUT = "/home/rsedor/Dev/ebpf-with-aya/presentation/ebpf-aya-101-r01.1.pptx";
const REV = "r01.1";

const pres = newDeck();
let pageNum = 0;
function S() { const s = pres.addSlide(); pageNum += 1; addFooter(s, pageNum); return s; }
function divider(code, title, subtitle, notes) {
  const s = pres.addSlide(); pageNum += 1; addSectionDivider(s, code, title, subtitle); addNotes(s, notes);
}

// ---- Cover -------------------------------------------------------------------
{
  const s = pres.addSlide(); pageNum += 1;
  s.background = { color: COLOR.white };
  try { s.addImage({ path: `${ASSETS}/cover-panel.png`, x: 0, y: 0, w: W, h: 7.5 }); } catch (e) {}
  s.addText("EBPF WITH AYA · 101", { x: 6.00, y: 1.98, w: 6.90, h: 0.34,
    fontFace: FONT.title, fontSize: 14, bold: true, color: COLOR.red, charSpacing: 6, align: "left", valign: "middle" });
  s.addText([{ text: "Safe kernel", options: { breakLine: true } }, { text: "code, in Rust" }], {
    x: 5.95, y: 2.42, w: 6.95, h: 2.00, fontFace: FONT.title, fontSize: 54, bold: true, color: COLOR.ink, align: "left", valign: "top" });
  s.addText("An introduction to eBPF and the Aya framework — for Rust developers.", { x: 6.00, y: 4.70, w: 6.70, h: 0.90,
    fontFace: FONT.body, fontSize: 18, italic: true, color: COLOR.caption, align: "left", valign: "top" });
  s.addText(REV, { x: 11.85, y: 5.85, w: 0.95, h: 0.30, fontFace: FONT.mono, fontSize: 11, color: COLOR.caption, align: "right", valign: "middle" });
  try { s.addImage({ path: `${ASSETS}/logo-candidate-2.png`, x: 11.10, y: 6.80, w: 1.55, h: 0.37 }); } catch (e) {}
  addNotes(s, "Welcome. This is the 101 — a ninety-minute, ground-up introduction to eBPF using Aya, the Rust framework. The audience I'm assuming is people comfortable in Rust who have never written kernel code. By the end you'll understand what eBPF is, the load-verify-attach model that every program follows, how Aya splits a project into a kernel half and a user-space half, and you'll have seen real programs that trace syscalls, watch functions, filter packets, and push what they see all the way to a Grafana dashboard. Everything here runs on a plain Fedora VM with Podman — no managed cloud, no special hardware. The deeper material — the verifier internals, CO-RE, container uprobes, the security and scheduling frontier — is the 201.");
}

// ============================================================ 01 · WHY
divider("01", "Why eBPF", "Programmable kernel, without kernel modules.",
  "Let's start with the why, because eBPF sounds like magic until you place it. The one-sentence version: eBPF lets you run small, safe, verified programs inside the running kernel, attached to events you care about, without writing a kernel module and without rebooting. That's the capability. The rest of this section is what that buys you.");

{
  const s = S();
  addContentTitle(s, "WHY · THE IDEA", "What eBPF actually is");
  addBullets(s, [
    "A tiny virtual machine in the kernel that runs your bytecode when an event fires.",
    "You attach a program to a hook — a syscall, a function entry, a packet arrival — and it runs there.",
    { text: "Safe by construction:", sub: "a verifier proves your program can't crash or hang the kernel before it ever runs." },
    { text: "Fast:", sub: "the bytecode is JIT-compiled to native instructions — kernel speed, not interpreter speed." },
    "No kernel module to build, sign, and load; no reboot. Attach and detach at runtime.",
  ], { fontSize: 17 });
  addNotes(s, "eBPF started life as the Berkeley Packet Filter — the thing tcpdump used to filter packets in-kernel — and grew into a general-purpose in-kernel virtual machine. The mental model to hold: there is a little VM inside the kernel, and you hand it bytecode plus a hook to attach to. When the hook fires — a process calls execve, a packet hits the NIC, a file gets opened — your code runs, in kernel context, with access to that event's data. Two properties make this usable in production. First, safety: unlike a kernel module, an eBPF program is checked by a verifier that rejects anything that could crash or hang the machine, so a bug is a load-time error, not a kernel panic. Second, speed: it's JIT-compiled to native code. And it's all dynamic — you attach and detach without rebooting, which is why observability and networking tools reach for it.");
}

{
  const s = S();
  addContentTitle(s, "WHY · THE PROBLEM", "The problem it solves for us");
  addBullets(s, [
    "You want to see what a running system is doing — syscalls, latency, GC pauses, packets — without changing the app.",
    { text: "Traditional answers are invasive:", sub: "recompile with instrumentation, inject an agent, parse verbose logs, or run a sidecar." },
    { text: "eBPF is out-of-process:", sub: "the signal comes from the kernel, so the target needs no code change and no agent inside it." },
    "One uniform mechanism across every process on the box — your app, someone else's binary, a container.",
    "The cost is paid only where you attach, and only while attached.",
  ], { fontSize: 17 });
  addNotes(s, "Here's the concrete pain eBPF removes. Say you want per-request latency inside a service, or you want to know which processes are opening a particular file, or why tail latency spikes. The classic options all touch the target: you recompile it with tracing, you load a language agent into the process, you turn on verbose logging and parse it, or you front it with a sidecar proxy. Every one of those changes the thing you're trying to observe. eBPF flips it: the observation happens in the kernel, underneath the process, so the process is untouched — no redeploy, no agent, no restart. And it's uniform: the same technique watches your Rust service, a stock Postgres binary, and a Java container, because they all make syscalls and run functions the kernel can see. You pay the overhead only at the hooks you attach, only while attached. That combination — total visibility, zero target changes — is why this is worth learning.");
}

{
  const s = S();
  addDiagramSlide(s, "WHY · THE PAYOFF", "What we'll build: kernel event to Grafana", "lgtm-pipeline",
    "The loader is also an OTLP exporter; the LGTM stack turns kernel truth into dashboards.");
  addNotes(s, "This is the arc of the whole course in one picture, so keep it in mind. On the left, an eBPF program runs in the kernel and a small user-space loader pulls its data out. That loader doubles as an OpenTelemetry exporter — it speaks OTLP over HTTP. It ships metrics and events to the LGTM stack: Loki for logs, Grafana for dashboards, Tempo for traces, and Mimir or Prometheus for metrics. The payoff on the right is a Grafana pane showing what the kernel actually saw — execve counts, GC pauses, HTTP latency — with no agent in the target and no change to the app. Every demo today ends here, at a dashboard. That's the point of pairing eBPF with the LGTM stack: kernel-level truth, presented like any other telemetry.");
}

// ============================================================ 02 · MODEL
divider("02", "The eBPF model", "Load, verify, attach — every program, the same shape.",
  "Now the model itself. The single most useful thing you can internalize today is that every eBPF program — no matter what it does — follows the same three-step lifecycle: load, verify, attach. Learn it once here and every chapter afterward is just a different hook and a different map.");

{
  const s = S();
  addDiagramSlide(s, "MODEL · LIFECYCLE", "Load → verify → attach", "ebpf-model",
    "Your Rust compiles to BPF bytecode; the loader submits it; the verifier gates it; the JIT runs it.");
  addNotes(s, "Walk left to right. You write the program in Rust and it compiles to BPF bytecode — a small, restricted instruction set. A user-space process, the loader, calls into the kernel to submit that bytecode; because loading eBPF is privileged, the loader runs with CAP_BPF. Before the kernel accepts it, the verifier analyzes every possible path through the program and proves it terminates and only touches memory it's allowed to — that's the amber box, the trust boundary. If it passes, the JIT compiles the bytecode to native machine code so it runs at full speed. Finally you attach the loaded program to a hook, and from then on it fires on that event. Load, verify, attach. The loader stays running to read results back out; when it exits, you can detach. Notice the split: the program runs in the kernel, but a normal user-space process drives the whole thing.");
}

{
  const s = S();
  addContentTitle(s, "MODEL · THE VERIFIER", "The verifier: why this is safe to run in the kernel");
  addBullets(s, [
    { text: "It simulates every path", sub: "through your program before load, tracking every register and stack slot." },
    { text: "Bounded loops only", sub: "— no unbounded iteration; the program must provably terminate." },
    { text: "Memory is checked", sub: "— every pointer access must be proven in-bounds; no wild reads or writes." },
    { text: "A finite instruction budget", sub: "— the analysis must complete, so programs have a complexity ceiling." },
    "Reject means a clear load-time error — not a kernel crash. You fix it and reload.",
  ], { fontSize: 16 });
  addCaption(s, "Fighting the verifier is a rite of passage; it is also what makes kernel programming safe for the rest of us.");
  addNotes(s, "The verifier is what makes this whole thing acceptable in a production kernel, so it's worth respecting. When you load a program, the verifier does abstract interpretation: it walks every path, tracking the possible values of every register and stack slot, and it enforces rules. Loops must be bounded — historically no loops at all, now bounded ones — because the program must provably finish; you can't hang the kernel. Every memory access must be proven within bounds before it happens, so you can't read or write arbitrary kernel memory. And the analysis itself has a budget, which puts a ceiling on program complexity. If any rule fails, load fails with an error message, and — this is the key part — nothing bad happened to the kernel; you just get a rejection and reload after fixing it. Everyone who writes eBPF spends time 'fighting the verifier.' Reframe that: the verifier is a proof assistant that refuses to let you write an unsafe kernel program. In Aya, a lot of that friction is handled by the API shape, but the model underneath is this.");
}

{
  const s = S();
  addContentTitle(s, "MODEL · PROGRAM TYPES", "A program type per kind of hook");
  addStatusTable(s, [
    { code: "tracepoint", name: "Stable kernel events", purpose: "Named, stable hooks the kernel exposes (e.g. syscalls:sys_enter_execve)." },
    { code: "kprobe", name: "Any kernel function", purpose: "Attach to (almost) any kernel function entry/return — powerful, less stable." },
    { code: "fentry/fexit", name: "Function entry/exit (BTF)", purpose: "The modern, low-overhead way to trace kernel functions with typed args." },
    { code: "uprobe", name: "User-space functions", purpose: "Attach to functions in a user binary or shared library (libssl, libjvm, your app)." },
    { code: "xdp / tc", name: "Networking", purpose: "Inspect/drop/redirect packets — XDP at the driver, tc in the stack." },
    { code: "lsm", name: "Security hooks", purpose: "Allow or deny an operation at a Linux Security Module hook." },
  ], { colW: [1.85, 3.05, 7.19], rowH: 0.62 });
  addNotes(s, "The 'type' of an eBPF program is really which family of hook it can attach to, and it determines what context and helpers you get. Tracepoints are named, stable events the kernel maintainers commit to — the safest tracing hook. Kprobes attach to almost any kernel function by name, which is enormously powerful but less stable across kernel versions, since internal functions come and go — we'll actually hit that later. fentry/fexit are the modern replacement for kprobes on function entry and exit: lower overhead and, thanks to BTF, typed access to the function's arguments. uprobes attach to functions in user-space binaries and libraries — that's how you trace OpenSSL, or a JVM, or your own app. XDP and tc are the networking types, one at the driver and one in the stack. And LSM programs sit at security hooks and return allow-or-deny. Don't memorize this table — just know the model is 'pick the hook family, get a program type.' We'll do one from most of these rows today.");
}

{
  const s = S();
  addDiagramSlide(s, "MODEL · MAPS", "Maps: how the kernel and your loader share data", "ebpf-maps",
    "The program writes; the loader reads. The map is the channel — no syscall per event.");
  addNotes(s, "A program that can't communicate is useless, and eBPF's communication mechanism is the map. A map is a data structure that lives in the kernel and is reachable from both sides: your eBPF program writes to it, and your user-space loader reads from it — or vice versa for configuration. The important design point is that you choose the map type to fit the job. A per-CPU array is great for counters you aggregate cheaply. A hash map keys data by pid or address. A ring buffer is an ordered stream of events. An array is how the loader passes configuration in. There's no syscall per event — the program updates the map in place, and the loader reads it on its own schedule. When you design an eBPF tool, 'which map' is one of your first two decisions, alongside 'which hook.' We'll use per-CPU arrays for counting and ring buffers for streaming today.");
}

{
  const s = S();
  addDiagramSlide(s, "MODEL · HOOKS", "One model, many surfaces", "ebpf-hooks",
    "Tracing, networking, security, scheduling — same load→verify→attach, different hook.");
  addNotes(s, "This ties the section together. The same three-step model points at wildly different parts of the kernel. Aim it at tracing hooks and you observe execution — what ran, how long it took, who opened what. Aim it at networking hooks and you filter or redirect packets. Aim it at LSM hooks and you make allow-or-deny security decisions. Aim it at perf and scheduling hooks and you measure or even influence how the CPU is shared. The book — and these talks — are organized exactly this way, as a tour of hook families. But the thing you carry from surface to surface is constant: write a small program, load it, let the verifier check it, attach it to a hook, and read a map from user space. Once the model clicks, learning a new capability is mostly learning a new hook and a new map.");
}

// ============================================================ 03 · AYA
divider("03", "Aya & the Rust workflow", "Rust on both sides of the kernel boundary.",
  "So that's eBPF in general. Now: how do we actually write it, in Rust, with Aya? Aya is a pure-Rust eBPF framework — no C, no libbpf dependency — and its big idea is that both the kernel program and the user-space loader are Rust, sharing types. Let's see the workflow.");

{
  const s = S();
  addContentTitle(s, "AYA · WHY", "Why Aya, and why Rust");
  addTwoColBullets(s,
    [
      { text: "Rust on both sides", options: { bullet: false, bold: true } },
      "Kernel program and loader are both Rust.",
      "Shared #[repr(C)] types — no manual struct duplication.",
      "no_std kernel crate; std loader crate.",
    ],
    [
      { text: "Pure Rust, no libbpf", options: { bullet: false, bold: true } },
      "No C toolchain, no bindgen against kernel headers.",
      "Cargo is the build system; bpf-linker links bytecode.",
      "Async loader with tokio, the ecosystem you know.",
    ],
    { fontSize: 16 });
  addCaption(s, "The pitch: one language, one build system, type-safe across the kernel boundary.");
  addNotes(s, "Why reach for Aya specifically? The historical way to write eBPF is C for the kernel program plus libbpf and often BCC for loading, glued to whatever language your userspace is in. Aya's pitch is: it's Rust all the way down, with no libbpf and no C toolchain. The kernel side is a no_std Rust crate compiled to the BPF target; the loader is an ordinary std Rust crate. Crucially, the types that cross the kernel-user boundary are defined once, with repr(C), in a shared crate compiled into both halves — so you're not hand-syncing a struct definition in two languages, which is a classic source of bugs. The build is just Cargo, with a special linker called bpf-linker producing the bytecode. And the loader is normal async Rust — you get tokio and the crates you already know. For a Rust shop, that's a big ergonomic and safety win over the C-plus-glue stack.");
}

{
  const s = S();
  addDiagramSlide(s, "AYA · WORKSPACE", "The two-crate workspace", "aya-workspace",
    "A kernel crate, a loader crate, and a shared-types crate compiled into both.");
  addNotes(s, "Here's how an Aya project is physically laid out, because it surprises people at first. It's one Cargo workspace with three crates. On the left, the -ebpf crate: that's the actual program the kernel runs, marked no_std because there's no standard library in the kernel, and built for the special BPF target with a nightly toolchain. On the right, the loader crate: ordinary std Rust on the stable toolchain, which loads, attaches, and reports. In the middle, a -common crate holding the repr(C) structs both sides exchange — it compiles into both. The glue is a build script using aya-build: at compile time it builds the -ebpf crate for the BPF target, bpf-linker links it, and the resulting object gets embedded into the loader binary. So when you ship, the loader carries the bytecode inside it. This layout is the one thing to get right when you scaffold a new project; after that it's just filling in the two src/main.rs files.");
}

{
  const s = S();
  addContentTitle(s, "AYA · TOOLCHAIN", "What you install once");
  addBullets(s, [
    { text: "A nightly toolchain + rust-src", sub: "— the BPF target needs to build core from source (-Z build-std)." },
    { text: "bpf-linker", sub: "— cargo install bpf-linker; links your bytecode into a loadable object." },
    { text: "A stable toolchain for the loader", sub: "— pinned per-crate via rust-toolchain.toml." },
    { text: "The lab: a Fedora VM with a recent kernel + BTF", sub: "— you build on the laptop, run on the VM." },
    "aya 0.14 / aya-ebpf 0.2 — the versions this course is built and verified against.",
  ], { fontSize: 16 });
  addCaption(s, "You never load eBPF on your laptop — you build there and deploy to the VM.");
  addNotes(s, "The one-time setup is small. The kernel crate targets BPF, which isn't a normal Rust target, so you need a nightly toolchain and rust-src to build the core library from source — that's the build-std flag. You install bpf-linker once with cargo install; it's what turns the compiled crate into a loadable BPF object. The loader uses a normal stable toolchain, and the two are pinned per-crate with a rust-toolchain.toml so the workspace just does the right thing. For the lab, we use a Fedora VM with a recent kernel that exposes BTF — I'll explain BTF properly in the 201, but it's the kernel's type information that modern eBPF leans on. The workflow, and this matters for safety and for your laptop's sanity, is: build on the laptop where the toolchain lives, then deploy the binary to the VM and load it there. eBPF never loads on your development machine. Everything today was built and verified against aya 0.14.");
}

{
  const s = S();
  addCodeSlide(s, "AYA · HELLO (KERNEL)", "hello-world: the program the kernel runs", "rust · aya-ebpf",
    [
      "#![no_std]",
      "#![no_main]",
      "use aya_ebpf::{macros::{map, tracepoint}, maps::PerCpuArray,",
      "               programs::TracePointContext};",
      "",
      "#[map] static EVENTS: PerCpuArray<u64> = PerCpuArray::with_max_entries(1, 0);",
      "",
      "#[tracepoint]                       // this is the hook family",
      "pub fn hello(_ctx: TracePointContext) -> u32 {",
      "    if let Some(ptr) = EVENTS.get_ptr_mut(0) {",
      "        unsafe { *ptr += 1; }       // count this execve",
      "    }",
      "    0                               // return 0 = OK",
      "}",
    ],
    "no_std, one map, one #[tracepoint] function — this is a complete eBPF program.");
  addNotes(s, "This is a complete, real eBPF program — the kernel half of hello-world. Top to bottom: no_std and no_main because we're in the kernel, not a normal binary. We import from aya-ebpf, the kernel-side crate. We declare one map with the #[map] attribute — a per-CPU array of one u64, which we'll use as a counter; per-CPU means each CPU has its own slot so there's no contention. Then the program itself: the #[tracepoint] macro marks this function as a tracepoint program — that's how we pick the hook family. The function gets a context, grabs a mutable pointer to the counter, and increments it. Returning zero means 'OK, continue.' That's it. Every time the hook fires — and we'll attach it to execve — this runs in the kernel and bumps a per-CPU counter. Notice there's no printf, no allocation, no syscall; it just touches a map. The unsafe block is pointer arithmetic into the map, which the verifier still checks. This is the shape of nearly every tracing program you'll write.");
}

{
  const s = S();
  addCodeSlide(s, "AYA · HELLO (LOADER)", "hello-world: the user-space loader", "rust · aya",
    [
      "let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(",
      "    concat!(env!(\"OUT_DIR\"), \"/hello\")))?;   // embedded bytecode",
      "",
      "let program: &mut TracePoint =",
      "    ebpf.program_mut(\"hello\").unwrap().try_into()?;",
      "program.load()?;                              // verify + JIT",
      "program.attach(\"syscalls\", \"sys_enter_execve\")?;   // hook it",
      "",
      "// read the per-CPU counter every second and export it",
      "let events: PerCpuArray<_, u64> = PerCpuArray::try_from(",
      "    ebpf.map_mut(\"EVENTS\").unwrap())?;",
      "let total: u64 = events.get(&0, 0)?.iter().sum();",
    ],
    "load → get program → load() (verify+JIT) → attach → read the map. The whole lifecycle in one screen.");
  addNotes(s, "And here's the other half — the loader, ordinary std Rust running on the VM. Line one: Ebpf::load takes the bytecode, which was embedded at build time via that OUT_DIR include — the bytes are baked into this binary. Then we look up the program by the name of our function, hello, and coerce it to a TracePoint. Calling load on it is where the verifier and JIT happen — this is the 'verify' step from our model. attach wires it to the syscalls sys_enter_execve tracepoint, so now every execve on the system runs our kernel code. Finally, the bottom two lines are the read side: we get a handle to the EVENTS map and sum the per-CPU slots to get the total. In the real program this runs in a loop, exporting the delta to OTLP every second. Put the two code slides together and you've seen an entire eBPF tool: a kernel program that counts, and a loader that attaches it and reads the count. This is genuinely the whole model, and everything else is variations.");
}

{
  const s = S();
  addContentTitle(s, "AYA · THE LAB", "Build on the laptop, run on the VM");
  addBullets(s, [
    { text: "cargo build --release on your machine", sub: "— fast CPU, warm caches, your editor; produces the loader binary with embedded bytecode." },
    { text: "deploy-to-target.sh ships it to the VM", sub: "and runs it under sudo (loading eBPF needs CAP_BPF)." },
    { text: "The VM exports OTLP back to the LGTM stack", sub: "running in a container on your laptop." },
    { text: "Each chapter is a demo.sh", sub: "— build, deploy, drive a workload, show the result in Grafana." },
    "You can even step-debug the loader running on the VM from your IDE over gdbserver.",
  ], { fontSize: 16 });
  addNotes(s, "One slide on how you actually work, because the split between laptop and VM is deliberate. You build on your laptop — fast CPU, your editor, warm caches — and that produces the loader binary with the bytecode embedded. A small script, deploy-to-target, copies it to the VM and runs it under sudo, because loading eBPF needs the CAP_BPF capability. The VM then exports its metrics over OTLP back to the LGTM observability stack, which runs as a container on your laptop. Every chapter in the book is a demo.sh that automates this: build, deploy, drive some workload so the hook fires, and show you the result in Grafana. The reason for the VM is safety and reproducibility — you never load experimental kernel programs on your daily driver, and everyone gets the same kernel. As a bonus, because the loader is just a Rust process on the VM, you can attach a remote debugger from your IDE and step through it line by line, which we cover in the toolchain chapter.");
}

// ============================================================ 04 · TRACING
divider("04", "Tracing", "Watch the system execute — syscalls, functions, user code.",
  "Now we start attaching to real hooks, and we begin with tracing, because it's where eBPF earns its keep for most people. Tracing means observing execution: which syscalls run, which kernel and user functions get called, how long they take. We'll walk the tracing hook families we saw in the table, with a real example for each.");

{
  const s = S();
  addContentTitle(s, "TRACING · TRACEPOINTS", "Tracepoints: stable, named kernel events");
  addBullets(s, [
    { text: "Kernel-maintained hooks with a stable name and layout", sub: "— e.g. syscalls:sys_enter_execve, sched:sched_switch." },
    { text: "The safest tracing hook", sub: "— the kernel commits to keeping them, so your tool survives upgrades." },
    { text: "That's exactly what hello-world used", sub: "— attach(\"syscalls\", \"sys_enter_execve\") counts every process launch." },
    { text: "Great for syscall-level and scheduler-level observability", sub: "— execs, opens, signals, context switches." },
    "List them on the VM: ls /sys/kernel/tracing/events.",
  ], { fontSize: 16 });
  addNotes(s, "Tracepoints are the friendliest place to start. They're hooks the kernel developers deliberately placed and named, with a documented, stable argument layout — things like the entry to the execve syscall, or the scheduler switching tasks. Because the kernel maintains them as a contract, a tool built on a tracepoint keeps working across kernel upgrades, which you can't always say for the deeper hooks. That's the hook hello-world used: attaching to sys_enter_execve means our counter increments once per process launch, system-wide. Tracepoints cover a huge amount of useful ground — every syscall has enter and exit tracepoints, and the scheduler, block layer, and networking subsystems expose them too. On the VM you can literally list them under /sys/kernel/tracing/events. When a tracepoint exists for what you want, use it before reaching for a kprobe.");
}

{
  const s = S();
  addContentTitle(s, "TRACING · KPROBES", "Kprobes: attach to (almost) any kernel function");
  addBullets(s, [
    { text: "Dynamic instrumentation of internal kernel functions", sub: "— by name, at entry (kprobe) or return (kretprobe)." },
    { text: "Enormously powerful", sub: "— if the kernel calls a function, you can probably watch it." },
    { text: "Less stable than tracepoints", sub: "— internal function names and signatures change between kernel versions." },
    { text: "A real gotcha we hit:", sub: "do_unlinkat was the classic file-deletion probe; on a newer kernel we retargeted to vfs_unlink." },
    "Reach for a tracepoint first; use a kprobe when there isn't one.",
  ], { fontSize: 16 });
  addCaption(s, "Power with a caveat: you're coupling to kernel internals, so pin your kernel and test on upgrades.");
  addNotes(s, "Kprobes are the power tool. Where tracepoints are a curated list, a kprobe attaches to almost any function inside the kernel, by name, either on entry or on return with a kretprobe. If the kernel calls it, you can usually watch it — which is incredible for debugging and for building tracers that go where no tracepoint exists. The trade-off is stability: these are internal functions, and their names and signatures aren't a stable interface, so they change across kernel versions. This isn't hypothetical — in building this course, the classic example for watching file deletion attached to a function called do_unlinkat, and on the newer kernel in our lab that function had changed enough that we retargeted the example to vfs_unlink instead. That's the kprobe tax: you're coupled to kernel internals, so you pin your kernel version and you retest on upgrades. The rule of thumb: if a tracepoint exists for what you need, prefer it; drop to a kprobe when it doesn't.");
}

{
  const s = S();
  addContentTitle(s, "TRACING · FENTRY/FEXIT", "fentry / fexit: the modern function hook");
  addBullets(s, [
    { text: "Attach at function entry (fentry) and exit (fexit)", sub: "— like kprobe/kretprobe, but lower overhead." },
    { text: "BTF gives you typed arguments", sub: "— the kernel's type info means you read args as real types, not raw offsets." },
    { text: "fexit sees both arguments and the return value", sub: "— perfect for 'did this succeed, and how long did it take?'" },
    { text: "Our vfs_unlink example uses fentry + fexit", sub: "— entry captures who/what; exit captures the result." },
    "This is the preferred way to trace kernel functions on a modern kernel.",
  ], { fontSize: 16 });
  addNotes(s, "fentry and fexit are the modern, preferred way to hook kernel functions, and they're a step up from kprobes in two ways. First, lower overhead — they use a more efficient trampoline mechanism. Second, and this is the big one, they use BTF, the kernel's type information, so you get typed access to the function's arguments. With a kprobe you're often reading raw registers and offsets; with fentry you read the actual argument as its real struct type, which the verifier understands. fexit is especially nice because at function exit you can see both the arguments and the return value in one place — which answers the two questions you usually have: did this operation succeed, and how long did it take? Our file-deletion example uses both: an fentry program captures who is deleting what, and the paired fexit program captures whether it succeeded. On a modern kernel with BTF, this is what you should reach for.");
}

{
  const s = S();
  addCodeSlide(s, "TRACING · FENTRY CODE", "fentry + fexit on vfs_unlink", "rust · aya",
    [
      "let btf = Btf::from_sys_fs()?;                 // the kernel's type info",
      "",
      "let enter: &mut FEntry =",
      "    ebpf.program_mut(\"vfs_unlink_enter\").unwrap().try_into()?;",
      "enter.load(\"vfs_unlink\", &btf)?;               // target fn + BTF",
      "enter.attach()?;",
      "",
      "let exit: &mut FExit =",
      "    ebpf.program_mut(\"vfs_unlink_exit\").unwrap().try_into()?;",
      "exit.load(\"vfs_unlink\", &btf)?;",
      "exit.attach()?;                                // fexit sees the return value",
    ],
    "Load the kernel's BTF, then load each program against the target function name.");
  addNotes(s, "Here's the loader side of that fentry/fexit example, and the new ingredient compared to hello-world is BTF. Line one loads the kernel's BTF — its type information — straight from the running kernel via sysfs. Then for each program we look it up, and when we call load we pass two things: the name of the kernel function we're targeting, vfs_unlink, and that BTF handle. The BTF is how Aya and the verifier know the real type of vfs_unlink's arguments, so the kernel-side program can read them as typed values. We do it twice: once for the entry program and once for the exit program, and then attach both. Now every call to vfs_unlink runs our entry code with the arguments, and every return runs our exit code with the result. The kernel-side code, which I'm not showing to keep this moving, reads the filename and the calling process on entry, and the success-or-failure code on exit, and streams that out. The pattern — load BTF, load against a function name, attach — is the whole fentry idiom.");
}

{
  const s = S();
  addContentTitle(s, "TRACING · UPROBES", "Uprobes: reach into user-space code");
  addBullets(s, [
    { text: "Attach to a function in a user binary or shared library", sub: "— not the kernel: libssl, libjvm, libpq, your own app." },
    { text: "See plaintext before encryption, queries before they hit the wire, GC pauses inside the JVM.",
      sub: "— because you're inside the process's own functions." },
    { text: "Resolve the symbol, attach a uprobe at its offset", sub: "— Aya reads the binary's symbol table for you." },
    { text: "Examples: sslsniff (OpenSSL), the Postgres query probe, JVM GC timing.",
      sub: "— all uprobes into a library or binary." },
    "This is how eBPF observes applications, not just the kernel.",
  ], { fontSize: 16 });
  addNotes(s, "Everything so far has been in the kernel. Uprobes point the same machinery at user space — you attach to a function inside a normal binary or shared library. This is a superpower for application observability, because you're running inside the process's own functions. Attach to OpenSSL's write function and you see plaintext before it's encrypted. Attach to Postgres's query executor and you see SQL before it touches the disk. Attach to the JVM's garbage collector and you time GC pauses. Mechanically, you resolve the function's symbol to an offset in the binary and attach a uprobe there — and Aya reads the binary's symbol table to do that resolution for you. The book has several of these: an SSL sniffer on libssl, a Postgres query-latency probe, and a JVM GC timer. Uprobes get gnarlier in production — stripped binaries, containers hiding the file — and that's a big topic in the 201. But the idea is simple and powerful: the same load-verify-attach model, aimed at your application's own code.");
}

{
  const s = S();
  addDiagramSlide(s, "TRACING · RING BUFFERS", "Streaming events with a ring buffer", "ringbuf-stream",
    "Reserve, fill, submit in the kernel; drain in a poll loop and export.");
  addNotes(s, "Counting with a per-CPU array is great for aggregates, but a lot of tracing produces a stream of individual events — one per file deleted, one per query, one per connection — and each carries fields you want. The right tool for that is a ring buffer. On the kernel side the pattern is reserve, fill, submit: you reserve space for your event struct in the ring, write the fields in place, and submit it. It's lock-free and allocation-free, and it preserves order. On the user side, the loader runs a poll loop calling next, decoding each event struct, and doing something with it — printing it, and in our case exporting it to OTLP. The ring buffer also handles back-pressure sanely: if the loader falls behind, the kernel side sees the reservation fail rather than corrupting anything. This is the modern replacement for the older perf-buffer approach, and it's what most of our streaming tracers use. So: arrays for counts, ring buffers for event streams.");
}

// ============================================================ 05 · NETWORKING
divider("05", "Networking", "Make decisions on packets — at the earliest point.",
  "Let's switch hook families entirely and look at networking, because it shows off a different superpower: acting on data, not just observing it. eBPF can inspect, drop, and redirect packets, and the flagship is XDP.");

{
  const s = S();
  addDiagramSlide(s, "NET · XDP", "XDP: decide before the stack even starts", "xdp-path",
    "Runs in the driver on packet arrival; return PASS, DROP, or redirect.");
  addNotes(s, "XDP — the eXpress Data Path — runs your program in the network driver, at the earliest possible moment, before the kernel even allocates its normal packet structure. Your program gets the raw packet, inspects the headers, and returns an action: PASS it up the stack, DROP it, or redirect it elsewhere. The reason people care is cost. If you're being flooded — a DDoS, or just unwanted traffic — dropping at XDP costs a handful of instructions per packet and the packet never touches the rest of the stack. That's how eBPF-based load balancers and DDoS scrubbers push millions of packets per second on commodity hardware. It's the same model you already know — write a small program, verify it, attach it — but the hook is the driver and the 'map' often holds a policy the loader updates live. Our demo just drops ICMP: once attached, pings to the box time out while everything else keeps working, and you can watch the drop counter climb in Grafana.");
}

{
  const s = S();
  addCodeSlide(s, "NET · XDP CODE", "An XDP program is a function returning an action", "rust · aya-ebpf",
    [
      "#[xdp]",
      "pub fn xdp_drop(ctx: XdpContext) -> u32 {",
      "    match try_inspect(&ctx) {",
      "        Ok(action) => action,",
      "        Err(_) => xdp_action::XDP_PASS,   // fail open",
      "    }",
      "}",
      "",
      "// inside: parse Ethernet/IP headers from the packet,",
      "// return xdp_action::XDP_DROP for ICMP, else XDP_PASS",
    ],
    "Same shape as a tracing program — a function over a context — but it returns a packet verdict.");
  addNotes(s, "And here's the shape of an XDP program, to show it's the same skeleton you already know. It's a function, marked with the #[xdp] macro this time, taking a context that wraps the raw packet, and returning a u32 — but now that u32 is a verdict: PASS, DROP, or REDIRECT. The body parses the packet headers — Ethernet, then IP — and decides. Notice the error handling returns XDP_PASS: if anything is malformed or we can't parse it, we fail open and let the packet through, which is the safe default for a filter. The real work is in the header parsing, which is careful about bounds because the verifier insists every byte you read is proven to be within the packet — you can't read past the end. That's the same verifier discipline as everywhere else, just applied to packet data. The takeaway: networking isn't a different mental model, it's the same function-over-a-context shape returning a different kind of answer.");
}

{
  const s = S();
  addContentTitle(s, "NET · TC/TCX", "tc / tcx: richer processing in the stack");
  addBullets(s, [
    { text: "XDP is ingress-only, at the driver; tc runs later, in the stack, on ingress and egress.",
      sub: "— by then the kernel's packet structure exists, so you have more context." },
    { text: "tcx is the modern, cleaner attach point", sub: "— composable, with a well-defined ordering of programs." },
    { text: "Use it to classify, shape, count, or redirect", sub: "— per-connection and per-cgroup policy lives here." },
    "Rule of thumb: XDP for cheap, early drops; tc/tcx for richer, stateful processing.",
  ], { fontSize: 16 });
  addNotes(s, "XDP is deliberately minimal and ingress-only — it runs before the kernel builds its rich packet representation, which is exactly why it's cheap. When you need more context, you use tc, the traffic-control layer, or its modern successor tcx. These run a bit later, in the network stack, on both ingress and egress, and by then the kernel's socket-buffer structure exists, so you have more to work with — connection state, metadata, the ability to act on outbound traffic too. tcx specifically is the newer, cleaner attach point with a well-defined ordering when multiple programs stack up, which matters when several tools want a say. You reach for tc when you're classifying traffic, shaping it, counting per-connection, or doing per-cgroup policy. The mental split is simple: XDP for the cheapest possible early drop, tc or tcx for richer, stateful processing where you need the kernel's full picture of the packet.");
}

// ============================================================ 06 · OBSERVABILITY
divider("06", "The observability payoff", "Kernel events, exported like any other telemetry.",
  "We've counted, traced, and filtered. The last section closes the loop we opened at the start: getting all of this into Grafana, so it's not terminal output but real dashboards you can alert on. This is where eBPF-with-Aya meets the LGTM stack.");

{
  const s = S();
  addCodeSlide(s, "OBS · EXPORT", "The loader is an OTLP exporter", "rust · opentelemetry",
    [
      "// the same loader that reads the map also exports metrics",
      "let meter = global::meter(\"ebpf-hello\");",
      "let counter = meter.u64_counter(\"ebpf_events_total\").build();",
      "",
      "loop {",
      "    let total: u64 = events.get(&0, 0)?.iter().sum();",
      "    counter.add(total - last, &[KeyValue::new(\"program\", \"hello\")]);",
      "    // OTLP/HTTP -> http://<gateway>:4318  -> LGTM stack",
      "}",
    ],
    "Read the kernel map, add the delta to an OTLP counter — kernel truth becomes a standard metric.");
  addNotes(s, "Here's the bridge, and it's satisfyingly ordinary. The same loader that reads the eBPF map is also a normal OpenTelemetry program. We create a meter and a counter metric called ebpf_events_total. Then in the loop, we read the per-CPU map, compute how much it grew since last time, and add that delta to the OTLP counter, tagged with which program produced it. Under the hood the OpenTelemetry SDK batches these and ships them over OTLP/HTTP to port 4318 on the gateway, where the LGTM stack receives them. The point is that there's nothing eBPF-specific about the export path — once you've pulled the number out of the kernel, it's just a metric, and you use the exact same OpenTelemetry code you'd use anywhere. That's why this composes so well: eBPF is the source, OpenTelemetry is the transport, Grafana is the destination, and they don't need to know about each other.");
}

{
  const s = S();
  addContentTitle(s, "OBS · THE STACK", "The LGTM stack, in one container");
  addStatusTable(s, [
    { code: "L", name: "Loki", purpose: "Log aggregation — structured logs from loaders and apps." },
    { code: "G", name: "Grafana", purpose: "The dashboards and Explore view — one pane over everything." },
    { code: "T", name: "Tempo", purpose: "Distributed traces — spans stitched across services (201)." },
    { code: "M", name: "Mimir / Prometheus", purpose: "Metrics at scale — where ebpf_events_total and friends land." },
  ], { colW: [1.10, 3.10, 7.89], rowH: 0.66 });
  addCaption(s, "grafana/otel-lgtm: the whole stack as a single container on your laptop.");
  addNotes(s, "LGTM is an initialism for the four Grafana components, and the reason we use it is that there's a single container image — grafana slash otel-lgtm — that bundles all of them preconfigured, so your whole observability backend is one docker run on your laptop. L is Loki, for logs. G is Grafana itself, the UI where you build dashboards and use the Explore view to poke at data. T is Tempo, for distributed traces — we lean on that in the 201 when we correlate across services. M is Mimir, which is Grafana's Prometheus-compatible metrics store, and that's where our ebpf_events_total metric lands. The beauty for a course like this is that you don't stand up four services; you run one container, point your loaders' OTLP endpoint at it, and everything shows up. In production these are separate scalable services, but the developer experience is identical, which is the whole idea.");
}

{
  const s = S();
  addContentTitle(s, "OBS · IN GRAFANA", "What you actually see");
  addBullets(s, [
    { text: "A live rate of kernel events", sub: "— e.g. sum(rate(ebpf_events_total[1m])) — execs per second, straight from the tracepoint." },
    { text: "Latency histograms", sub: "— function or request latency as a heatmap and p99 line, from ring-buffer events." },
    { text: "Per-target labels", sub: "— break down by program, by container, by result — the kernel event, sliced." },
    { text: "Alerts on kernel truth", sub: "— 'GC pause p99 > 10ms', 'ICMP drops climbing' — signals the app can't measure about itself." },
    "Same dashboards you'd build for app metrics — the source just happens to be the kernel.",
  ], { fontSize: 16 });
  addNotes(s, "So what do you actually get in Grafana? First, live rates — a one-line PromQL query turns our counter into execs-per-second, plotted over time, sourced from a kernel tracepoint. Second, latency histograms — the ring-buffer tracers emit per-event durations, and those become heatmaps and p99 lines, which is the classic 'is latency getting worse' view. Third, labels — because we tag metrics by program, by container, by success or failure, you can slice the same kernel event however you need, and compare across services on one dashboard. And fourth, the real payoff, alerting on things the application literally cannot measure about itself: a GC pause that froze every thread, a climbing packet-drop rate, a syscall storm. The dashboards look exactly like the ones you'd build for ordinary application metrics — same queries, same panels — but the data originates in the kernel, out-of-process, uniformly across everything on the box. That's the whole thesis of pairing eBPF with LGTM.");
}

{
  const s = S();
  addContentTitle(s, "CLOSE · RECAP", "What you now know");
  addBullets(s, [
    "eBPF runs safe, verified, JIT-compiled programs in the kernel, attached to hooks.",
    "Every program follows the same shape: load → verify → attach, and shares data through maps.",
    "Aya writes both halves in Rust — a no_std kernel crate and a std loader — sharing repr(C) types.",
    "You've seen tracing (tracepoint, kprobe, fentry, uprobe), networking (XDP, tc), and the export path.",
    "The loader doubles as an OTLP exporter, so kernel events become Grafana dashboards.",
  ], { fontSize: 16 });
  addNotes(s, "Let's recap the arc. One: eBPF is safe, verified, JIT-compiled code that runs in the kernel attached to hooks, giving you out-of-process visibility with no changes to the target. Two: every program, whatever its job, follows the same load-verify-attach lifecycle and communicates through maps — internalize that and the whole field opens up. Three: Aya lets you write both the kernel program and the loader in Rust, sharing types across the boundary, with Cargo as the build system. Four: you've now seen the major tracing hooks — tracepoints, kprobes, fentry, uprobes — plus networking with XDP and tc, all as the same shape aimed at different surfaces. And five: because the loader is just a Rust process, it doubles as an OpenTelemetry exporter, so everything the kernel sees can land in Grafana as a normal metric. That's a real, working mental model of production eBPF — enough to read the code, run the demos, and start building.");
}

{
  const s = S();
  addContentTitle(s, "CLOSE · WHERE NEXT", "Where to go from here — the 201");
  addTwoColBullets(s,
    [
      { text: "Go deeper", options: { bullet: false, bold: true } },
      "The verifier and CO-RE/BTF, properly.",
      "Maps in depth: per-CPU, ring buffers, perf, pinning.",
      "Uprobes into stripped container binaries.",
    ],
    [
      { text: "Go wider", options: { bullet: false, bold: true } },
      "Security: LSM allow/deny, signals, file protection.",
      "The frontier: kfuncs, struct_ops, sched_ext, timers.",
      "Where Aya hits its limits vs libbpf — and why.",
    ],
    { fontSize: 16 });
  addCaption(s, "Clone the repo, bring up the VM, and run demo.sh in any chapter — every example here is runnable and verified.");
  addNotes(s, "Where do you go from here? Two directions, both in the 201. Deeper: we take the things I hand-waved today — the verifier, and CO-RE and BTF which make programs portable across kernels — and treat them properly; we go through the map types in real depth; and we tackle the genuinely hard version of uprobes, reaching into stripped binaries hidden inside containers, which is where production application tracing actually lives. And wider: security programs that make allow-or-deny decisions at LSM hooks, and the frontier — kfuncs, struct_ops, the sched_ext scheduler, BPF timers — plus a candid look at where Aya still hits limits compared to libbpf and what to do about it. The best next step, though, is hands-on: clone the repo, bring up the VM, and run demo.sh in any chapter. Every example in this course is runnable and was verified on real hardware. Thank you — let's take questions.");
}

pres.writeFile({ fileName: OUT })
  .then(p => console.log("WROTE", p))
  .catch(e => { console.error(e); process.exit(1); });
