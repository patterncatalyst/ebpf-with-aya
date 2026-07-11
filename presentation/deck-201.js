// deck-201.js — "eBPF with Aya · 201" (~3h, senior devs, standalone + recap).
// Build: export NODE_PATH=$(npm root -g) && node deck-201.js
"use strict";

const H = require("./deck-helpers.js");
const {
  COLOR, FONT, W, ASSETS,
  newDeck, addFooter, addContentTitle, addBullets, addTwoColBullets,
  addStatusTable, addCaption, addCodeSlide, addDiagramSlide, addSectionDivider, addNotes,
} = H;

const OUT = "/home/rsedor/Dev/ebpf-with-aya/presentation/ebpf-aya-201-r01.1.pptx";
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
  s.addText("EBPF WITH AYA · 201", { x: 6.00, y: 1.98, w: 6.90, h: 0.34,
    fontFace: FONT.title, fontSize: 14, bold: true, color: COLOR.red, charSpacing: 6, align: "left", valign: "middle" });
  s.addText([{ text: "The hard parts,", options: { breakLine: true } }, { text: "in depth" }], {
    x: 5.95, y: 2.42, w: 6.95, h: 2.00, fontFace: FONT.title, fontSize: 54, bold: true, color: COLOR.ink, align: "left", valign: "top" });
  s.addText("The verifier, CO-RE, container uprobes, security, and the frontier — with Aya.", { x: 6.00, y: 4.70, w: 6.90, h: 0.90,
    fontFace: FONT.body, fontSize: 18, italic: true, color: COLOR.caption, align: "left", valign: "top" });
  s.addText(REV, { x: 11.85, y: 5.85, w: 0.95, h: 0.30, fontFace: FONT.mono, fontSize: 11, color: COLOR.caption, align: "right", valign: "middle" });
  try { s.addImage({ path: `${ASSETS}/logo-candidate-2.png`, x: 11.10, y: 6.80, w: 1.55, h: 0.37 }); } catch (e) {}
  addNotes(s, "Welcome to the 201. This is the three-hour, senior-developer companion to the 101. It stands alone — the next few slides recap the fundamentals fast enough that you can attend cold — but it moves quickly, and it goes deep. We'll take the verifier and CO-RE seriously, treat maps as a design space, do the genuinely hard version of uprobes into stripped container binaries, cover security enforcement with LSM, tour the frontier where eBPF implements kernel behavior instead of just observing it, and be candid about where Aya still hits limits versus libbpf. Everything is grounded in real, verified examples from the book, running on a plain Fedora VM. Let's go.");
}

// ============================================================ 00 · RECAP
divider("00", "Fundamentals, fast", "The 101 in ten minutes — enough to stand alone.",
  "A compressed recap so this deck is self-contained. If you sat through the 101, treat this as a warm-up; if you didn't, this is the model everything else builds on. Four ideas: the lifecycle, the verifier, maps, and Aya's two-crate shape.");

{
  const s = S();
  addDiagramSlide(s, "RECAP · MODEL", "Every program: load → verify → attach", "ebpf-model",
    "Rust → BPF bytecode → loader submits → verifier gates → JIT → attached to a hook.");
  addNotes(s, "The whole field reduces to this lifecycle. You write a small program that compiles to BPF bytecode; a privileged user-space loader submits it; the verifier proves it's safe before the kernel accepts it; the JIT turns it into native code; and you attach it to a hook where it fires on events. The loader stays running to read results out through maps. Load, verify, attach — memorize it, because every advanced topic today is a variation on one of these steps: harder verification, exotic hooks, richer maps.");
}

{
  const s = S();
  addContentTitle(s, "RECAP · THE PIECES", "The four things to hold in your head");
  addTwoColBullets(s,
    [
      { text: "The program", options: { bullet: false, bold: true } },
      "Runs in the kernel, verified + JITed.",
      "Attached to a hook: tracepoint, kprobe, XDP, LSM…",
      { text: "The map", options: { bullet: false, bold: true } },
      "Shared kernel/user memory — the data channel.",
    ],
    [
      { text: "The loader", options: { bullet: false, bold: true } },
      "User-space Rust; loads, attaches, reads maps.",
      "Doubles as an OTLP exporter to Grafana.",
      { text: "The types", options: { bullet: false, bold: true } },
      "repr(C) structs shared across the boundary.",
    ],
    { fontSize: 15 });
  addNotes(s, "Four moving parts. The program is the kernel-side code at a hook. The loader is the user-space process that loads it and reads its output — and in this course it's also an OpenTelemetry exporter, so kernel data becomes Grafana metrics. The map is the shared-memory channel between them. And the types crossing that boundary are defined once as repr(C) structs. Aya's distinctive move is that the program, the loader, and the shared types are all Rust, in one workspace. Hold these four and everything today has a place to land.");
}

{
  const s = S();
  addCodeSlide(s, "RECAP · AYA SHAPE", "A program and its loader, minimal", "rust · aya",
    [
      "// kernel side (#![no_std], aya-ebpf)",
      "#[tracepoint]",
      "pub fn hello(_ctx: TracePointContext) -> u32 { /* touch a map */ 0 }",
      "",
      "// user side (std, aya)",
      "let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(",
      "    concat!(env!(\"OUT_DIR\"), \"/hello\")))?;",
      "let p: &mut TracePoint = ebpf.program_mut(\"hello\").unwrap().try_into()?;",
      "p.load()?;                                   // verify + JIT",
      "p.attach(\"syscalls\", \"sys_enter_execve\")?;   // hook it",
    ],
    "The kernel program is a macro-annotated fn; the loader loads, coerces, load()s (verify), attaches.");
  addNotes(s, "This is the whole shape in one slide. Up top, a kernel program — a function tagged with a program-type macro, no_std, that touches a map and returns. Below, the loader: it loads the embedded bytecode, looks the program up by name, coerces it to the right program type, calls load which is where verification and JIT happen, and attaches it to a hook. Everything else in this deck is a richer version of one of these lines: a harder program, a trickier attach, or a fancier map. If this reads cleanly to you, you're ready for the rest.");
}

{
  const s = S();
  addContentTitle(s, "RECAP · WHAT'S NEW HERE", "What the 201 adds");
  addBullets(s, [
    "The verifier and CO-RE/BTF — treated as first-class, not hand-waved.",
    "Maps as a design space — per-CPU, ring buffers, perf, pinning, sharing.",
    "Uprobes into stripped binaries inside containers — the real production problem.",
    "Security enforcement (LSM), and the frontier: struct_ops, sched_ext, kfuncs, timers.",
    "A candid map of where Aya still needs C — and how to work with that.",
  ], { fontSize: 16 });
  addNotes(s, "Here's the itinerary and how it differs from the 101. We take the verifier and CO-RE — the portability machinery — seriously instead of hand-waving them. We treat maps as a set of design decisions with real trade-offs. We do the hard uprobe problem: reaching into a stripped binary hidden inside a rootless container, which is where application tracing actually lives in production. We move from observing to enforcing with LSM, and we tour the frontier where eBPF implements kernel behavior. And we end with a clear-eyed boundary — where Aya is all-Rust and where the ecosystem still speaks C first — because you'll hit that edge and you should know how to handle it. Every claim is backed by a runnable, verified example.");
}

// ============================================================ 01 · VERIFIER & CO-RE
divider("01", "The verifier & CO-RE", "Safety you fight, portability you rely on.",
  "Two pieces of machinery decide whether your program loads at all and whether it survives a kernel upgrade: the verifier and CO-RE. Understanding them turns 'mysterious rejection' and 'works on my kernel' into things you can reason about.");

{
  const s = S();
  addDiagramSlide(s, "VERIFIER · INSIDE", "How the verifier decides", "verifier-detail",
    "Abstract interpretation over every path, register ranges tracked, states pruned, budget enforced.");
  addNotes(s, "The verifier is a static analyzer that runs at load time. It does abstract interpretation: it walks every reachable path through your program, and for each register and stack slot it tracks the set of values it could hold — a range, a pointer-to-map-value, a pointer-to-packet with known bounds. At every memory access it checks that the pointer is provably in range. It enforces bounded loops and that you don't leak kernel pointers to user space. Two practical facts fall out. First, state pruning: the verifier remembers equivalent states so the path explosion stays tractable — but it's not free. Second, there's an instruction-complexity budget; a big monolithic program can exceed it, which is why you split logic across tail calls and helper functions. When it rejects, you get an error and a partial trace, and nothing bad happened to the kernel.");
}

{
  const s = S();
  addContentTitle(s, "VERIFIER · WHAT TRIPS YOU UP", "The rejections you'll actually hit");
  addBullets(s, [
    { text: "Unbounded or too-large loops", sub: "— bound them with a const limit the verifier can see; use #[unroll] or bpf_loop." },
    { text: "Unchecked pointer arithmetic", sub: "— every packet/map read must be proven in-bounds first; the classic XDP header-parse dance." },
    { text: "Reading a possibly-null map value", sub: "— match on the Option; the verifier tracks that you checked." },
    { text: "Programs too complex for the budget", sub: "— split across tail calls; keep helpers small." },
    { text: "A real one from the book:", sub: "a zero-length bpf_skb_load_bytes got rejected — we clamped the length to at least 1." },
  ], { fontSize: 15 });
  addNotes(s, "Concretely, here's what rejects your program. Loops the verifier can't bound — you fix them by making the bound a constant it can see, or using bpf_loop. Pointer arithmetic it can't prove safe — in XDP you literally check 'is there room for an Ethernet header' before reading it, then 'room for IP' before that, and the verifier follows along. Reading a map value without handling the null case — in Aya you match on the Option, and the verifier sees the check. Programs that blow the complexity budget — split them. And a very real example from building this course: on the newer kernel, a call to bpf_skb_load_bytes with a length the verifier couldn't prove was nonzero got rejected, and the fix was to clamp the length to at least one. That's the texture of verifier work — small, provable guarantees that unblock the load.");
}

{
  const s = S();
  addDiagramSlide(s, "CO-RE · PORTABILITY", "CO-RE: one binary across kernels", "btf-core",
    "BTF describes each kernel's types; the loader relocates field offsets at load — no recompile.");
  addNotes(s, "CO-RE — Compile Once, Run Everywhere — solves the problem that kernel struct layouts differ between versions. Without it, a program that reads task_struct->pid is baked to a specific byte offset, and a kernel that moved that field breaks it. With CO-RE, the compiler records that you want the pid field symbolically, and at load time the loader consults the target kernel's BTF — its type information — and rewrites the access to the correct offset for that kernel. One binary, many kernels, no recompilation. BTF is the enabling technology: it's the kernel describing its own types to userspace, exposed at slash-sys-slash-kernel-slash-btf-slash-vmlinux. This is why modern eBPF leans on a BTF-enabled kernel, and it's what makes fentry's typed arguments possible too.");
}

{
  const s = S();
  addContentTitle(s, "CO-RE · IN AYA", "BTF in the Aya workflow");
  addBullets(s, [
    { text: "fentry/fexit load against the kernel's BTF", sub: "— Btf::from_sys_fs(), then load(\"vfs_unlink\", &btf) — typed args for free." },
    { text: "aya-tool generates Rust bindings from BTF", sub: "— turns kernel types into Rust structs you can read with CO-RE relocations." },
    { text: "Requires a BTF-enabled kernel", sub: "— /sys/kernel/btf/vmlinux; every recent Fedora/RHEL kernel ships it." },
    { text: "C reference path uses vmlinux.h", sub: "— bpftool btf dump file /sys/kernel/btf/vmlinux format c — same BTF, different front end." },
    "The kprobe caveat still applies: a renamed function breaks you even with CO-RE — CO-RE relocates fields, not function names.",
  ], { fontSize: 15 });
  addNotes(s, "How BTF shows up in Aya, concretely. fentry and fexit programs load against the kernel's BTF, which you grab with Btf::from_sys_fs, and that's what gives you typed access to the traced function's arguments. For CO-RE reads of kernel structs, aya-tool generates Rust bindings from BTF so you can read fields with proper relocations. All of this needs a BTF-enabled kernel, which every recent Fedora and RHEL kernel is. On the C reference side — the frontier chapters — you generate a vmlinux.h header from the same BTF with bpftool and include it; it's the same information, just consumed by Clang instead of rustc. One important caveat: CO-RE relocates field offsets, not function names. So the kprobe stability problem — do_unlinkat getting renamed to vfs_unlink between kernels — is not something CO-RE saves you from. Field moved: CO-RE handles it. Function renamed or removed: you retarget.");
}

// ============================================================ 02 · MAPS
divider("02", "Maps in depth", "The design space between kernel and user space.",
  "Maps are where a lot of eBPF design actually happens. Pick the wrong one and you get contention, lost events, or a syscall per record. Let's go through the map zoo and when each is right.");

{
  const s = S();
  addDiagramSlide(s, "MAPS · THE CHANNEL", "The program writes, the loader reads", "ebpf-maps",
    "Map type is a design decision: aggregation, streaming, config, or lookup.");
  addNotes(s, "Recall the shape: a map lives in the kernel and both sides touch it. The design question is which map. The four jobs are aggregation — counting cheaply; streaming — ordered event delivery; configuration — the loader pushing settings in; and lookup — keying data by pid or address for correlation. Each has a best-fit type, and getting it wrong shows up as contention, dropped events, or overhead. Let's make the choices explicit.");
}

{
  const s = S();
  addContentTitle(s, "MAPS · THE ZOO", "Choosing the right map");
  addStatusTable(s, [
    { code: "PerCpuArray", name: "Cheap counters", purpose: "One slot per CPU — no contention; loader sums across CPUs. Aggregation." },
    { code: "HashMap", name: "Keyed state", purpose: "Key by pid/tid/addr — entry/exit timing, in-flight tracking, correlation." },
    { code: "RingBuf", name: "Event streams", purpose: "Ordered, lock-free, back-pressure aware — the modern event channel." },
    { code: "Array", name: "Config in", purpose: "Loader writes settings the program reads — target pid, thresholds, flags." },
    { code: "PerfEventArray", name: "Legacy streams", purpose: "The older per-CPU perf-buffer path; RingBuf supersedes it for most uses." },
    { code: "Stack/LRU/…", name: "Specialized", purpose: "Stack traces, LRU eviction, longest-prefix (routing), maps-of-maps." },
  ], { colW: [2.15, 2.60, 7.34], rowH: 0.60 });
  addNotes(s, "The map zoo, and when to reach for each. PerCpuArray: counters with zero contention — each CPU owns a slot, the loader sums them; this is how hello-world counts. HashMap: keyed state — the entry/exit pattern stores a start timestamp keyed by pid or tid and looks it up on exit to compute a duration; it's also how you track in-flight operations. RingBuf: the modern ordered event stream, lock-free and back-pressure aware — most streaming tracers use it. Array: configuration flowing the other way, loader to program — a target pid, a threshold. PerfEventArray is the older per-CPU streaming path that RingBuf largely replaces, though you'll still see it. And then specialized types: stack-trace maps for profiling, LRU hashes for bounded caches, longest-prefix-match for routing tables, and maps-of-maps for per-cgroup structures. Pick by the job, not by habit.");
}

{
  const s = S();
  addContentTitle(s, "MAPS · PINNING & SHARING", "Maps that outlive a process");
  addBullets(s, [
    { text: "Pin a map to bpffs", sub: "— /sys/fs/bpf/… gives it a filesystem path that survives the loader exiting." },
    { text: "Share state across programs and tools", sub: "— one program writes, another (or bpftool) reads the same pinned map." },
    { text: "In Aya: map_pin_path on the loader", sub: "— then bpftool map dump pinned /sys/fs/bpf/NAME cross-checks it live." },
    { text: "Pinning also decouples lifecycle", sub: "— load once, attach many; restart the reader without losing the map." },
    "This is how you build multi-component tools and how bpftool inspects your maps.",
  ], { fontSize: 15 });
  addNotes(s, "By default a map dies when its last user goes away — usually when your loader exits. Pinning changes that: you give the map a path in the BPF filesystem, bpffs, mounted at slash-sys-slash-fs-slash-bpf, and now it persists independently. Two big uses. One, sharing: one program writes the map and a completely separate program — or bpftool from the command line — reads it by that path. Two, lifecycle decoupling: you can load and pin once, then restart the user-space reader without tearing down the kernel state. In Aya you set a pin path on the map when loading, and then you can literally run bpftool map dump on the pinned path to see your data live, which is invaluable for debugging. The pin-demo chapter walks through exactly this. Pinning is the mechanism behind multi-component eBPF tools and behind ground-truth inspection with bpftool.");
}

{
  const s = S();
  addDiagramSlide(s, "MAPS · RING BUFFERS", "Ring buffers, properly", "ringbuf-stream",
    "reserve → fill → submit in the kernel; drain in a poll loop; back-pressure is visible.");
  addNotes(s, "Ring buffers deserve a closer look because they're the workhorse for event streams. The kernel-side API is reserve, fill, submit: you reserve space for your event struct, write the fields in place — no copy — and submit, which makes it visible to userspace. If the buffer is full because the reader fell behind, the reserve fails and you handle it, rather than silently corrupting or blocking; that's the back-pressure being visible. The user side polls and drains in order. Compared to the older per-CPU perf buffer, the ring buffer is a single shared buffer with proper ordering and lower overhead, and it doesn't force you to reason per-CPU. The one thing to watch is sizing — too small and you drop under load, too large and you waste memory. For nearly all new streaming code, ring buffer is the right default.");
}

// ============================================================ 03 · ADVANCED TRACING
divider("03", "Advanced tracing", "Where uprobes get real: stripped binaries in containers.",
  "Tracing kernel functions is the easy case. The hard, valuable case is tracing application code — a database, a web server, a JVM — when it's a stripped binary running inside a rootless container. This section is the production reality of uprobes.");

{
  const s = S();
  addDiagramSlide(s, "UPROBE · THE HARD CASE", "Uprobing inside a rootless container", "container-uprobe",
    "The container hides the binary; the symbols are split out. Bind-mount a copy, merge the debug info.");
  addNotes(s, "Here's the problem in full. You want to uprobe a function inside, say, Postgres running in a rootless podman container. Two things fight you. First, rootless podman keeps the container's binary out of the host mount namespace, so your loader — which runs as root on the host — literally cannot see the file to attach to. Second, the production binary is stripped, and its symbols live in a separate dbgsym package as a split-debug file linked by a build-id. So you can't resolve the function's address. The fixes, both used in the book's Postgres and nginx chapters: for visibility, extract a copy of the binary and bind-mount it into the container so the container executes a host-visible inode you can also see. For symbols, merge the split-debug info back into that binary with eu-unstrip, because a bare split-debug file has a NOBITS text section with no real offsets, which the loader rejects. After the merge, the symbol sits in the binary's own symbol table with a real file offset, and Aya resolves it. This is fiddly, and it's exactly what real application tracing requires.");
}

{
  const s = S();
  addContentTitle(s, "UPROBE · SYMBOL RESOLUTION", "Why a bare .debug isn't enough");
  addBullets(s, [
    { text: "Stripped binary + split-debug via .gnu_debuglink", sub: "— the symbols are in a separate file keyed by build-id." },
    { text: "A split .debug has a NOBITS .text", sub: "— it carries symbols but no code offsets; 'symbol has no offset' on attach." },
    { text: "eu-unstrip merges .debug back into the binary", sub: "— now symbols land in .symtab with real .text offsets." },
    { text: "Aya reads .symtab and .dynsym", sub: "— pass the (mangled, for C++) symbol name; Aya computes the file offset." },
    "Same trick rescued the JVM GC example: uprobe G1CollectedHeap::do_collection_pause_at_safepoint from libjvm's .symtab.",
  ], { fontSize: 15 });
  addNotes(s, "Let me make the symbol problem precise, because it bit us repeatedly. A stripped production binary keeps a tiny link — a build-id in a .gnu_debuglink section — pointing at a separate .debug file that holds the symbols. The catch: that .debug file's text section is NOBITS — it declares the symbols but has no actual code, so there are no real file offsets to attach a uprobe to, and you get 'symbol has no offset.' The fix is eu-unstrip, which merges the debug info back into a real binary so the symbols land in a normal symbol table with real offsets. Aya's resolver reads both the dynamic symbol table and the full symbol table, so once the symbol is really there, you just hand Aya the name — the mangled name, for C++ — and it computes the offset. This exact technique also rescued the JVM garbage-collection example: a stock OpenJDK has no USDT GC markers, but its libjvm keeps a full symbol table, so we uprobe the G1 collector's pause function by its mangled name straight from that table. Symbol resolution is half the battle in application tracing.");
}

{
  const s = S();
  addContentTitle(s, "UPROBE · USDT VS. SYMBOLS", "USDT is ideal — when it exists");
  addTwoColBullets(s,
    [
      { text: "USDT (the ideal)", options: { bullet: false, bold: true } },
      "Author-placed stable markers (.note.stapsdt).",
      "Survives version changes; the documented surface.",
      "But: needs the binary built for it.",
      "Fedora's OpenJDK is NOT --enable-dtrace.",
    ],
    [
      { text: "Uprobe the function (the fallback)", options: { bullet: false, bold: true } },
      "When the marker isn't there, probe the real fn.",
      "Resolve from .symtab; accept version coupling.",
      "How javagc actually works on a stock JDK.",
      "Always check: readelf -n … | grep <probe>.",
    ],
    { fontSize: 15 });
  addNotes(s, "A nuance that trips people up: USDT versus raw uprobes. USDT — user statically-defined tracepoints — are stable markers the author baked into the binary, described in a stapsdt ELF note. They're the ideal surface for runtime-internal events like GC or query execution, because they survive version changes. But they only exist if the binary was built with them, and here's the gotcha we hit: a stock Fedora OpenJDK is not built with dtrace support, so its hotspot GC USDT markers simply don't exist. The portable fallback is to uprobe the actual C++ function the marker would have sat in, resolved from the symbol table — accepting that you're now coupled to that version's internals. That's how the javagc example really works. The lesson: aspire to USDT, but always check with readelf whether the markers are actually present, and have the symbol-table fallback ready, because on real distro binaries they often aren't.");
}

{
  const s = S();
  addCodeSlide(s, "UPROBE · ATTACH BY SYMBOL", "Aya resolves the symbol for you", "rust · aya",
    [
      "let sym = \"_ZN15G1CollectedHeap32do_collection_pause_at_safepointEm\";",
      "",
      "let begin: &mut UProbe =",
      "    ebpf.program_mut(\"gc_begin\").unwrap().try_into()?;   // #[uprobe]",
      "begin.load()?;",
      "begin.attach(sym, &libjvm, UProbeScope::AllProcesses)?;  // entry",
      "",
      "let end: &mut UProbe =",
      "    ebpf.program_mut(\"gc_end\").unwrap().try_into()?;     // #[uretprobe]",
      "end.load()?;",
      "end.attach(sym, &libjvm, UProbeScope::AllProcesses)?;    // return",
    ],
    "A uprobe on entry and a uretprobe on return, both at one symbol — Aya reads .symtab and does the offset math.");
  addNotes(s, "Here's the attach code, from the retargeted JVM example. We pass Aya the mangled C++ symbol name for the G1 pause function. We attach two programs at the same symbol: a uprobe on entry, marked with the uprobe macro on the kernel side, and a uretprobe on return, marked with uretprobe. Aya's attach takes the symbol name, the path to libjvm, and a scope — all processes here — and internally it reads the binary's symbol table, finds the symbol, and computes the correct file offset, including the virtual-address-to-file-offset translation. The entry program timestamps into a hash map keyed by thread id; the return program looks it up and computes the pause. That's a complete, stock-JDK GC latency tool built purely from symbol resolution — no USDT required. Note the API shape: attach takes a symbol, not a raw offset, and the resolver does the rest.");
}

// ============================================================ 04 · fentry/kfuncs
divider("04", "fentry, BTF & kfuncs", "Typed kernel access — and its current edge in Aya.",
  "Back in the kernel, the modern function-tracing story is fentry/fexit with BTF, and the frontier of it is kfuncs — calling kernel functions from your program. This is also where we hit a concrete Aya limitation, which is worth seeing clearly.");

{
  const s = S();
  addContentTitle(s, "FENTRY · TYPED, CHEAP", "fentry/fexit over kprobes");
  addBullets(s, [
    { text: "Lower overhead than kprobes", sub: "— a BTF-based trampoline instead of a breakpoint trap." },
    { text: "Typed arguments via BTF", sub: "— read the real struct, not a raw register; the verifier understands the type." },
    { text: "fexit sees args AND return in one place", sub: "— 'did it succeed, and how long did it take?' in a single program." },
    { text: "Our vfs_unlink example", sub: "— fentry captures who/what on entry; fexit captures the result code." },
    "Prefer fentry/fexit on any BTF-enabled kernel; drop to kprobe only when you must.",
  ], { fontSize: 15 });
  addNotes(s, "fentry and fexit are the modern way to hook kernel functions, and they beat kprobes on two axes. Overhead: they use a BTF-based trampoline rather than a breakpoint trap, so they're cheaper. And ergonomics: because they load against BTF, you read the function's arguments as their real kernel types, and the verifier understands those types, so field access is checked and CO-RE relocations apply. fexit is especially powerful because at exit you have both the arguments and the return value in one program — the two things you usually want together. Our file-deletion example splits it: an fentry program on vfs_unlink records who is deleting which file, and the paired fexit program records whether the deletion succeeded. On any BTF kernel, reach for these first; kprobes are the fallback for when there's genuinely no better hook.");
}

{
  const s = S();
  addContentTitle(s, "KFUNCS · AND AN AYA LIMIT", "kfuncs: calling into the kernel");
  addBullets(s, [
    { text: "kfuncs are kernel functions the kernel explicitly exports to BPF", sub: "— typed, allow-listed, the modern alternative to fixed helper IDs." },
    { text: "Acquire/release pairs enforce discipline", sub: "— e.g. bpf_task_from_pid / bpf_task_release, checked by the verifier." },
    { text: "The Aya edge:", sub: "aya-ebpf doesn't emit the BTF call relocation for arbitrary kfuncs — 'function not found' at load." },
    { text: "The book's task example works around it", sub: "— use a stable helper (bpf_get_current_pid_tgid) instead of a kfunc lookup, and document why." },
    "kfuncs are where the C reference still leads; know the boundary so you don't fight it.",
  ], { fontSize: 15 });
  addNotes(s, "kfuncs are the modern way the kernel exposes functionality to BPF programs — instead of a fixed numbered helper, the kernel exports typed functions with an allow-list, and some come in acquire/release pairs that the verifier enforces, like taking a reference to a task struct and being required to release it. They're powerful. But this is one place Aya has a concrete, current limitation: aya-ebpf doesn't emit the BTF-based call relocation that arbitrary kfuncs need, so a program that tries to call one fails to load with 'function not found,' and there's no kfunc-declaration mechanism yet. The book's task example runs straight into this — the canonical C version looks up an arbitrary pid's task with a kfunc pair — and works around it by using a stable, always-available helper instead, with a written explanation of exactly why the kfunc path doesn't work in Aya today. The takeaway for a senior audience: know this boundary. When you need real kfunc calls, that's a C reference program plus an Aya observer, not an all-Aya solution — for now.");
}

// ============================================================ 05 · NETWORKING
divider("05", "Networking, deeper", "XDP, tc/tcx, and stateful packet processing.",
  "Networking is where eBPF acts rather than observes, and where performance is the whole point. We covered the basics in the 101; here we go into the layers, the modes, and stateful processing like load balancing.");

{
  const s = S();
  addDiagramSlide(s, "NET · THE LAYERS", "XDP vs tc/tcx: where and when", "xdp-path",
    "XDP at the driver, ingress-only, cheapest; tc/tcx in the stack, both directions, richer.");
  addNotes(s, "The networking hooks form a layered pipeline. XDP runs first, in the driver, before the kernel builds its socket buffer — ingress only, and the cheapest possible place to act, which is why DDoS drops and fast load balancers live here. tc, and its modern successor tcx, run later in the stack where the full packet structure exists, on both ingress and egress, so you get more context and can act on outbound traffic. The design rule: XDP for the earliest, cheapest verdict; tc or tcx when you need connection state, egress, or to cooperate with other programs. tcx specifically fixes tc's old attachment mess with a clean, ordered, composable attach point — important when multiple tools want a say on the same interface.");
}

{
  const s = S();
  addContentTitle(s, "NET · XDP MODES & STATE", "XDP in practice");
  addBullets(s, [
    { text: "Three modes", sub: "— native (driver, fast), offload (on the NIC, fastest, rare), generic (software fallback, slow but universal)." },
    { text: "Actions", sub: "— PASS, DROP, TX (bounce back out), REDIRECT (to another NIC/CPU/socket)." },
    { text: "State lives in maps", sub: "— a load balancer keeps its backend table in a map the loader updates live; no reload to change policy." },
    { text: "Bounds are everything", sub: "— every header read is proven against the packet end; the verifier is strict here." },
    "The book's xdp-lb keeps a backend map and rewrites/redirects — policy is data, not code.",
  ], { fontSize: 15 });
  addNotes(s, "XDP in practice has a few things worth knowing. There are three attach modes: native, running in the driver, which is the normal fast path; offload, running on a capable SmartNIC, which is fastest but rare hardware; and generic, a software fallback that works everywhere but is slow — useful for development on a VM. The action set is PASS, DROP, TX which bounces the packet back out the same NIC, and REDIRECT which sends it to another interface, CPU, or socket. The important architectural point is that state lives in maps: a load balancer keeps its backend list in a map that the user-space loader updates live, so you change policy — add a backend, drain one — without reloading the program. Policy is data, not code. And the verifier is at its strictest here: every single header field read must be proven within the packet bounds first, which is why XDP parsing code has that careful, layered bounds-checking structure. The book's load-balancer example is exactly this: a backend map, header rewriting, and redirect.");
}

// ============================================================ 06 · SECURITY
divider("06", "Security & enforcement", "From observing to deciding: LSM and friends.",
  "Everything so far observes or forwards. Security programs decide — they return allow or deny at a kernel security hook. This is a different posture with real consequences, and Aya supports it fully.");

{
  const s = S();
  addDiagramSlide(s, "SEC · LSM", "LSM: allow or deny in the kernel", "lsm-decision",
    "An eBPF program at an LSM hook returns 0 (allow) or -EPERM (deny) — enforcement, not observation.");
  addNotes(s, "LSM — the Linux Security Module framework — is where SELinux and AppArmor plug in, and eBPF can plug in there too. At an LSM hook — checking an exec, a file open, a signal, a socket connect — your program runs and returns a value, and that value is a decision: zero to allow, negative-EPERM to deny. This is a categorical shift from tracing. A tracing program that has a bug produces bad data; an LSM program that has a bug denies legitimate operations or allows bad ones. So you write these carefully, you test the allow path as hard as the deny path, and you keep a way to disable them. But the capability is remarkable: policy-as-code, attached and detached at runtime, with the full context of the operation, uniform across the whole system. The enabling requirement is that the kernel is booted with BPF LSM active, which our lab does with a boot parameter.");
}

{
  const s = S();
  addContentTitle(s, "SEC · WHAT YOU CAN ENFORCE", "The security chapters");
  addStatusTable(s, [
    { code: "lsm-confine", name: "Egress by cgroup", purpose: "Deny network connects from a confined cgroup — allow the host, block the container." },
    { code: "signal-kill", name: "Block signals", purpose: "Deny SIGKILL to a protected process at the task_kill hook." },
    { code: "fileprotect", name: "Immutable files", purpose: "Deny writes to a protected path — READ-OK, WRITE-DENIED, in-kernel." },
    { code: "pidhide", name: "Understand rootkits", purpose: "Hide a pid from /proc by rewriting getdents64 — taught defensively, to recognize the technique." },
    { code: "sudoadd", name: "Forge privilege", purpose: "Rewrite the bytes sudo reads from /etc/sudoers to grant root — disk untouched; taught to detect." },
  ], { colW: [2.05, 2.75, 7.29], rowH: 0.62 });
  addNotes(s, "The security part of the book is a tour of enforcement. lsm-confine denies outbound network connections from a specific cgroup — so a confined container can't phone home while the host still can, decided at the socket hook. signal-kill denies SIGKILL to a protected process at the task_kill hook — you literally can't kill it, from the kernel's point of view. fileprotect makes a path effectively immutable by denying writes at the file hook — reads succeed, writes are denied, no matter who you are. pidhide is taught deliberately as defense: it shows how a malicious probe could hide a process, so you can recognize the technique — this is the one place we lean adversarial, to build intuition for what rootkits do. And sudoadd is the offense counterpart to those defenses: it forges the very bytes sudo reads from /etc/sudoers, granting a chosen user root while the file on disk stays pristine — taught so you can detect it (a loaded read tracepoint you can enumerate; effective privileges that disagree with the on-disk policy). It's also a candid lesson in kernel-version reality: on this kernel the original tripped the verifier — bpf_probe_write_user's size argument must be provably non-zero — and a naive 'tamper every sudo read' corrupted sudo's own shared-library loads and bricked it; both are fixed now, by clamping the length and targeting only the sudoers header. Together they show eBPF as an enforcement layer, not just a lens.");
}

{
  const s = S();
  addContentTitle(s, "SEC · DOING IT RESPONSIBLY", "Enforcement needs guardrails");
  addBullets(s, [
    { text: "Test the allow path as hard as the deny path", sub: "— a bug here breaks legitimate work, not just your data." },
    { text: "Keep an escape hatch", sub: "— detach/reboot recovery; our lab reverts to a clean snapshot." },
    { text: "Fail open vs fail closed is a decision", sub: "— decide deliberately what an unparseable case does." },
    { text: "Scope tightly", sub: "— by cgroup, by pid, by path; a system-wide deny is a big hammer." },
    "These are dual-use techniques — the book frames them for defense and authorized testing.",
  ], { fontSize: 15 });
  addNotes(s, "Because enforcement has teeth, it needs guardrails, and this is worth saying to a senior audience directly. Test your allow path as rigorously as your deny path — the failure mode of a security program is breaking legitimate work, and that's an outage. Always keep an escape hatch: a way to detach, and a recovery path if you wedge the box — in our lab that's reverting to a clean snapshot, which is exactly why we built the snapshot tooling. This isn't hypothetical: building the sudoers-forging example, a naive version corrupted sudo's own shared-library reads and locked us out of sudo entirely — you couldn't even sudo to kill the offending program, and a VM reboot was the only way back. Decide fail-open versus fail-closed deliberately for the cases your program can't classify. Scope as tightly as you can — by cgroup, pid, or path — because a system-wide deny is a very big hammer. And note these are dual-use techniques; the book frames them squarely for defense and authorized testing, and the pidhide chapter in particular is there so you can recognize an attack, not launch one. Enforcement is powerful; treat it with the seriousness it deserves.");
}

// ============================================================ 07 · FRONTIER
divider("07", "The frontier", "When eBPF implements the kernel, not just watches it.",
  "The most exciting — and least settled — part of eBPF is where programs stop observing and start implementing kernel behavior: scheduling, congestion control, timers. This is also where Aya's kernel-side authoring is still catching up, so it's where the C reference matters most.");

{
  const s = S();
  addDiagramSlide(s, "FRONTIER · THE MAP", "Program types beyond tracing & networking", "frontier",
    "struct_ops, sched_ext, kfuncs, timers, user_ringbuf, iterators, syscall programs.");
  addNotes(s, "Here's the frontier at a glance. struct_ops lets a BPF program implement a kernel vtable — most famously a TCP congestion-control algorithm you register and the kernel calls. sched_ext goes further: a whole CPU scheduler written in BPF that you can swap in at runtime. kfuncs, which we already met, are the typed kernel functions programs call. bpf_timer gives you kernel-side timers with callbacks that fire without user space involvement. user_ringbuf is a ring buffer the other direction — user space producing to the kernel, consumed via dynptr callbacks. BPF iterators walk kernel objects and let you cat, say, a process table the kernel assembled. And syscall programs are the loader programs behind light skeletons. What unites these is that several are where Aya's kernel-side story is still emerging in 2026, so the reference implementations are C loaded with bpftool, often with an Aya observer alongside. Let's hit the ones that matter most.");
}

{
  const s = S();
  addContentTitle(s, "FRONTIER · struct_ops & sched_ext", "Implementing kernel policy in BPF");
  addBullets(s, [
    { text: "struct_ops: fill in a kernel vtable", sub: "— a TCP congestion-control algorithm as BPF; register with bpftool struct_ops, appears in tcp_available_congestion_control." },
    { text: "sched_ext: a pluggable CPU scheduler", sub: "— implement enqueue/dispatch in BPF, attach it, and the kernel schedules by your policy; detach to fall back." },
    { text: "Both are C-first today", sub: "— cc.bpf.c and scx_*.bpf.c, loaded with their tooling; the Aya crate is a companion observer." },
    { text: "Verified in the book", sub: "— the CC algorithm registers and serves traffic; the scheduler attaches and runs a workload." },
    "This is eBPF at its most audacious: swap core kernel behavior, live, without a reboot.",
  ], { fontSize: 15 });
  addNotes(s, "Two headline frontier capabilities. struct_ops lets you implement a kernel interface — a vtable — in BPF. The canonical example is TCP congestion control: you write the ssthresh, cong_avoid, and undo_cwnd callbacks, register the whole thing with bpftool struct_ops, and it shows up in the kernel's list of available congestion-control algorithms alongside cubic and reno, selectable per socket. We verified exactly that on the lab kernel — including fixing the reference C when a kernel BTF change altered a helper's signature. sched_ext is even bolder: you implement a CPU scheduler's core decisions in BPF, attach it, and the kernel schedules tasks by your policy until you detach, at which point it falls back to the default — a safety valve that makes experimenting sane. Both of these are C-first in 2026: the scheduler and the congestion-control vtable are .bpf.c files loaded with their own tooling, and Aya rides along as an observer. But the capability is stunning: you're swapping core kernel behavior, live, no reboot.");
}

{
  const s = S();
  addContentTitle(s, "FRONTIER · timers, user_ringbuf, iterators", "The rest of the frontier");
  addBullets(s, [
    { text: "bpf_timer", sub: "— schedule a callback in the kernel; aya-ebpf can't yet emit the timer BTF, so the book computes rate in user space and documents why." },
    { text: "user_ringbuf", sub: "— user→kernel queue drained via a dynptr callback; works in Aya 0.14 with allow_unsupported_maps()." },
    { text: "BPF iterators", sub: "— a SEC(\"iter/task\") program builds a process table in-kernel; cat the pin to read it. C reference, verified (469 tasks)." },
    { text: "syscall / loader programs", sub: "— BPF_PROG_TYPE_SYSCALL is what a light skeleton embeds; inspectable with bpftool gen skeleton -L." },
    "Each ships with a candid verification note: what works in Aya, what needs the C reference, and why.",
  ], { fontSize: 15 });
  addNotes(s, "Rounding out the frontier. bpf_timer schedules a callback inside the kernel that fires on its own — but aya-ebpf can't yet emit the BTF a timer needs, so the book's timer example computes its rate in user space instead and documents the limitation plainly. user_ringbuf is the reverse ring buffer, user space producing and the kernel consuming via a dynptr callback; that one does work in Aya 0.14, with a loader flag to allow the not-yet-fully-supported map type. BPF iterators are lovely: a program tagged iter/task walks every task in the kernel and emits a row, you pin it, and cat the pin to get a process table the kernel assembled — we verified it producing 469 tasks. And syscall programs are the loader programs that light skeletons embed; you can inspect one with bpftool gen skeleton dash-L, which was its own adventure since Aya's objects use a legacy map layout that the modern skeleton generator rejects. The through-line: every one of these chapters carries a plain note about what works in Aya versus what needs the C reference, and why.");
}

// ============================================================ 08 · AYA LIMITS
divider("08", "Where Aya meets its edge", "A candid boundary, and how to work with it.",
  "A senior talk owes you candor about limits. Aya is excellent and improving fast, but in 2026 there's a real boundary between what's all-Rust and what still needs C. Knowing where it is saves you from fighting the tool.");

{
  const s = S();
  addDiagramSlide(s, "LIMITS · THE MAP", "All-Rust today vs. C reference still leads", "aya-c-boundary",
    "Tracing, networking, security, maps, perf — all Rust. The newest kernel surface still speaks C first.");
  addNotes(s, "Here's the boundary as of 2026. On the left, everything that's fully all-Rust in Aya today, and it's most of what you'll do: all of tracing — tracepoints, kprobes, fentry, uprobes; all of networking — XDP, tc, tcx, socket ops; security with LSM; every map type; ring buffers; perf events. That's the overwhelming majority of production eBPF, and it's pure Rust with no C in the loop. On the right, where the C reference still leads: struct_ops and sched_ext, because implementing kernel vtables and schedulers in aya-ebpf is still emerging; BPF iterators and arena; and kfunc call relocations, which aya-ebpf can't emit yet. The pattern on the right is consistent — a canonical .bpf.c loaded with bpftool or the subsystem's tooling, with an Aya crate as a companion. This isn't Aya being deficient; it's that the newest kernel surface lands in C first and Aya catches up. The practical value is knowing which side of this line your task is on before you start.");
}

{
  const s = S();
  addContentTitle(s, "LIMITS · WORKING WITH IT", "Practical strategies at the edge");
  addBullets(s, [
    { text: "Rust fill-ins where the kernel-side gap is small", sub: "— e.g. compute a rate in user space instead of a bpf_timer; use a stable helper instead of a kfunc." },
    { text: "C reference + Aya observer where it isn't", sub: "— the scheduler/CC vtable is C; your Aya program watches and exports its effects." },
    { text: "Pin the versions", sub: "— aya 0.14 / aya-ebpf 0.2 here; the API moved meaningfully from 0.13, and it's still moving." },
    { text: "Read the verification note first", sub: "— every frontier chapter states what's verified, on what kernel, and what's approximate." },
    "The gap shrinks every release — check maintenance status before assuming a limit still holds.",
  ], { fontSize: 15 });
  addNotes(s, "So how do you actually work at that edge? Two strategies depending on how big the gap is. When the kernel-side gap is small, do a Rust fill-in: the book computes a timer's rate in user space rather than using bpf_timer, and uses a stable pid helper instead of a task-lookup kfunc — you keep the lesson and stay all-Rust, and you document the substitution. When the gap is fundamental — a scheduler, a congestion-control vtable — you accept a C reference for the kernel part and let your Aya program be the observer that measures and exports its effects. Pin your versions: this course is aya 0.14 and aya-ebpf 0.2, and the API moved meaningfully from the 0.13 line — attach signatures, the log init order, map pinning all changed — so version discipline matters. Always read a chapter's verification note first; it tells you what's verified, on which kernel, and what's approximate. And crucially, this boundary moves every release, so before you treat a limit as fixed, check the current maintenance status — what was C-only last year may be Rust today.");
}

// ============================================================ 09 · OBSERVABILITY
divider("09", "Production observability", "Three signals, correlated, from the kernel up.",
  "We close where the 101 closed, but deeper: not just a metric on a dashboard, but metrics, logs, and traces correlated — kernel-level truth stitched into distributed traces across services. This is the production payoff.");

{
  const s = S();
  addDiagramSlide(s, "OBS · CORRELATION", "Tying kernel events to distributed traces", "correlation",
    "The W3C traceparent flows through services; eBPF tags kernel spans with the same trace id.");
  addNotes(s, "This is the capstone idea. In a distributed system, a request carries a W3C traceparent header from service to service, and each service records spans into Tempo. Normally those spans stop at the application boundary — you see 'service B took 200ms' but not why. eBPF closes that gap: a probe sees the syscall or the database query underneath the request and can tag its measurement with the same trace id that's flowing through the app. Now the trace in Grafana includes kernel-level spans — the actual disk wait, the actual lock contention — stitched into the same timeline as the application spans. The three signals join on the trace id: a metric spike in Mimir, the log line in Loki, and the trace in Tempo all point at the same request, and the kernel evidence sits right underneath the application's own story. That's observability that no in-process agent can give you, because the agent can't see below itself.");
}

{
  const s = S();
  addDiagramSlide(s, "OBS · THE PIPELINE", "Loader → OTLP → LGTM → Grafana", "lgtm-pipeline",
    "The loader is an OTLP exporter; the LGTM stack is one container; Grafana is the pane.");
  addNotes(s, "The transport is deliberately boring, and that's the point. Your loader — already a Rust process reading maps — is also an OpenTelemetry exporter, emitting metrics, logs, and trace spans over OTLP. The LGTM stack receives them: Loki, Grafana, Tempo, and a Prometheus-compatible metrics store, all bundled in one container image for the lab. Grafana is the single pane. There's nothing eBPF-specific about this half — once the data is out of the kernel, it's ordinary telemetry, which is exactly why it composes with everything else in your observability estate. In production these are scaled-out services and the loader might export to a collector first, but the shape is identical: kernel is the source, OpenTelemetry is the transport, Grafana is the destination.");
}

{
  const s = S();
  addContentTitle(s, "OBS · THE THREE SIGNALS", "What eBPF contributes to each");
  addStatusTable(s, [
    { code: "Metrics", name: "Mimir / Prom", purpose: "Rates and latency histograms from counters and ring-buffer durations — ebpf_events_total, p99 GC pause." },
    { code: "Logs", name: "Loki", purpose: "Structured events — each unlink, connect, or denied operation as a queryable line." },
    { code: "Traces", name: "Tempo", purpose: "Kernel spans correlated by trace id — the syscall/query latency under a request." },
  ], { colW: [1.85, 2.55, 7.69], rowH: 0.72 });
  addCaption(s, "The same kernel event can feed all three — aggregate as a metric, record as a log, correlate as a span.");
  addNotes(s, "Concretely, what does eBPF put into each of the three signals? Metrics: the per-CPU counters become rates, and the ring-buffer event durations become latency histograms — execs per second, p99 GC pause, drop counts — landing in the Prometheus-compatible store. Logs: each discrete event — a file deleted, a connection denied, a query executed — can be emitted as a structured log line into Loki, queryable after the fact. Traces: the latency an eBPF probe measures for a syscall or a query becomes a span in Tempo, correlated by trace id to the application's own spans. The elegant part is that a single kernel event can feed all three depending on what you need — aggregate it as a metric, record it as a log, correlate it as a span. The book's three-signals and correlation chapters build exactly this, so you can go from a Grafana alert down to the kernel evidence in a couple of clicks.");
}

{
  const s = S();
  addContentTitle(s, "CLOSE · THE THROUGH-LINE", "What the 201 leaves you with");
  addBullets(s, [
    "The verifier and CO-RE are reasonable systems — you can predict rejections and portability.",
    "Maps are a design space; pinning turns them into shared, inspectable infrastructure.",
    "Real application tracing is a symbol-resolution problem — bind-mounts, eu-unstrip, .symtab.",
    "eBPF enforces (LSM) and implements (struct_ops, sched_ext), not just observes — with C still leading the newest surface.",
    "Kernel truth joins the three signals in Grafana, correlated by trace id — the payoff no in-process agent can match.",
  ], { fontSize: 15 });
  addNotes(s, "Let's pull the through-line together. The verifier and CO-RE stop being magic: you can predict why something is rejected and reason about whether it'll survive a kernel upgrade. Maps are a design space, and pinning promotes them to shared, bpftool-inspectable infrastructure. Real application tracing turns out to be mostly a symbol-resolution problem — bind-mounting a container's binary, merging split-debug with eu-unstrip, resolving from the symbol table — and once you've internalized that, stripped binaries in containers stop being scary. eBPF isn't just a lens: it enforces with LSM and implements kernel behavior with struct_ops and sched_ext, with the candid caveat that the newest surface still leads in C. And it all lands in the same three-signal observability stack, correlated by trace id, giving you kernel-level evidence stitched into distributed traces — which is genuinely something no in-process agent can do.");
}

{
  const s = S();
  addContentTitle(s, "CLOSE · GO BUILD", "Where to go from here");
  addBullets(s, [
    "Clone the repo, bring up the VM (lab-up.sh), and run demo.sh in any chapter — all verified on Fedora 44 / kernel 7.1.3.",
    "Start on the all-Rust side: a tracepoint or fentry tool that exports one metric you care about.",
    "When you hit the C boundary, use it as a signal — an Aya observer beside a C reference is a fine architecture.",
    "Snapshot the VM before the security and scheduler chapters; revert-vm.sh is your undo.",
    "Watch aya's release notes — the edge you hit today may be gone next version.",
  ], { fontSize: 15 });
  addCaption(s, "Every example in this course is runnable and was verified on real hardware — the code is the ground truth.");
  addNotes(s, "Finally, how to take this further. Clone the repo, bring the VM up with lab-up, and run any chapter's demo.sh — every example was verified on Fedora 44 with kernel 7.1.3, so the code is the ground truth, not the prose. Start on the all-Rust side: build a small tracepoint or fentry tool that exports one metric you actually care about, and wire it to your Grafana. When you hit the C boundary, don't treat it as failure — an Aya observer running beside a C reference program is a perfectly good production architecture, and it keeps most of your code in Rust. Before you run the security and scheduler chapters, snapshot the VM, because those can wedge it — and revert-vm is your one-command undo. And keep an eye on Aya's releases; this is a fast-moving project and the specific limitations I've been candid about are actively being closed. That's the 201 — thank you, and let's take questions.");
}

pres.writeFile({ fileName: OUT })
  .then(p => console.log("WROTE", p))
  .catch(e => { console.error(e); process.exit(1); });
