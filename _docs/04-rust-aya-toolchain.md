---
title: "The Rust + Aya toolchain"
order: 4
part: Foundations
description: "Install Rust 1.96.0 via rustup (not dnf), add the nightly BPF target and bpf-linker, scaffold projects with cargo-generate and the aya-template, and set up RustRover. Plus the background that pays off later: the LLVM/Clang/rustc compiler path that turns code into BPF bytecode, the wider eBPF tooling landscape, and the glibc-vs-musl choice for your loader binary."
duration: 35 minutes
---

This chapter installs everything needed to *build* eBPF programs in
Rust on your laptop. Aya is unusual and pleasant here: for the great majority of this book
the kernel-side and user-side are both Rust — no Clang in the loop —
and a release build produces one self-contained binary you ship to the
target VM. (A handful of advanced chapters are the exception, where a
kernel feature Aya can't yet author is written in C; we flag it plainly
each time, and the section below lists them.) But the eBPF half compiles to a special target with a
special linker, so the setup has a few moving parts. We'll get them
all in place and prove them with a build in Chapter 6.

{% include excalidraw.html
   file="workspace-build"
   alt="The three-crate workspace: a common crate of shared types, an ebpf crate kept out of the default build (compiled instead by build.rs via aya-build into a BPF object), and a loader crate that embeds the object with include_bytes_aligned."
   caption="Figure 4.1 — the three-crate workspace and build flow" %}

## Why rustup, not `dnf install rust`

Fedora's packaged Rust is fine for ordinary programs, but eBPF
compilation needs **two toolchains at once** — a pinned stable for
user space and a **nightly** for the BPF target (the BPF code uses
unstable `build-std` features) — and it needs them switchable
per-crate. That's exactly what `rustup` is for. We install via
[rustup.rs](https://rustup.rs) and leave Fedora's system Rust alone.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

Then load it into the current shell (zsh users: rustup writes to
`~/.zshenv` automatically, but the current session needs the path
now):

```bash
. "$HOME/.cargo/env"
```

## Pin the Rust version

This tutorial pins **Rust 1.96.0** for the user-space side. Install
and default to it:

```bash
rustup toolchain install 1.96.0 && rustup default 1.96.0 && rustc --version
```

> **A timing note (read this).** At the time these chapters were
> written — late May 2026 — `1.96.0` is the **beta** channel and goes
> stable on or about **2026-06-05**. Stable *today* is `1.95.0`. If
> `rustup toolchain install 1.96.0` can't find a stable 1.96.0 yet,
> you're ahead of its release: either use `rustup default 1.95.0` for
> now and bump later, or install the beta with `rustup toolchain
> install beta`. Once 1.96.0 ships stable, the command above is
> correct as written. The tutorial pins 1.96.0 per the project
> decision; the reconciliation plan records this nuance.

Rather than rely on the global default, we pin per-project with a
`rust-toolchain.toml` (the `aya-template` generates one). That file is
what makes RustRover and `cargo` agree on the version without you
thinking about it.

## Add the nightly toolchain and the BPF target

The eBPF crate is compiled with nightly and a rebuilt `core`:

```bash
rustup toolchain install nightly --component rust-src && rustup component add rust-src --toolchain nightly
```

`rust-src` is what lets nightly rebuild `core` for the BPF target via
`-Z build-std=core`. You don't run that flag by hand — the
template's build script (an `xtask`) does — but the component must be
present.

## Install bpf-linker

eBPF object files are linked by **`bpf-linker`**, a Rust tool that
wraps LLVM's BPF backend. Install it from crates.io:

```bash
cargo install bpf-linker
```

`bpf-linker` needs LLVM available at build time. If the install fails
complaining about LLVM, install Fedora's LLVM development packages and
retry — kept, as a policy, from Fedora repositories:

```bash
sudo dnf install -y llvm llvm-devel clang && cargo install bpf-linker --no-default-features
```

(On most Fedora 44 setups the plain `cargo install bpf-linker` just
works because it bundles a compatible LLVM; the `--no-default-features`
form links against the system LLVM you just installed, which is the
fallback.)

## What's actually compiling this: LLVM, Clang, and rustc

eBPF is a **bytecode** — a small, verifiable instruction set the kernel runs in
its in-kernel virtual machine. Nothing writes that bytecode by hand; a compiler
backend emits it, and for eBPF that backend is **LLVM's BPF target**. Every
mainstream eBPF toolchain, C or Rust, ultimately funnels through it. What
differs is the *front end*:

- **C eBPF** — the libbpf/BCC world most kernel docs assume — uses **Clang**, an
  LLVM C front end, to compile a `.bpf.c` file straight to a BPF object:
  `clang -O2 -g -target bpf -c prog.bpf.c -o prog.o`. The `-g` matters: it keeps
  the BTF debug info that CO-RE relies on (Chapter 58).
- **Rust eBPF** — Aya — uses **rustc**, also an LLVM front end, to emit BPF, and
  then **`bpf-linker`** drives LLVM's BPF backend to produce and finalize the
  object. So in the *Rust* path there is no Clang at all: rustc + bpf-linker
  replace it end to end. That's the "both halves are Rust" pleasantness from
  this chapter's opening, made concrete.

Which is why this book installs *both*. Your Aya programs never call Clang — but
the wider eBPF ecosystem is C, so the **reference programs** alongside the
frontier chapters are `.bpf.c` compiled with Clang, and two CO-RE essentials
need it too: generating `vmlinux.h` (`bpftool btf dump … format c`, then
`#include`d by a Clang build) and **`aya-tool`**, which leans on **bindgen**
(hence libclang) to turn kernel BTF into Rust bindings. So `clang` and `llvm`
earn their place in the prerequisites even though your day-to-day Rust builds
don't touch Clang.

**One candid caveat, and the real reason Clang is required.** For the great
majority of this book the kernel-side program is Rust — but a handful of
advanced chapters are the exception, where the *in-kernel* program that actually
runs is **C**, because Aya's kernel-side authoring for that specific feature is
still emerging in 2026. The user-space side stays Rust or `bpftool`; the kernel
program is C, loaded with `bpftool` or the subsystem's own tooling:

- **sched_ext schedulers** (Chapters 43–44) — the scheduler is `scx_*.bpf.c`,
  loaded via the `scx` tooling; the Aya crate there is a companion *observer*,
  not the scheduler itself.
- **struct_ops** (Chapter 55) — the TCP congestion-control vtable is `cc.bpf.c`,
  installed with `bpftool struct_ops register`.
- **BPF iterators** (Chapter 57) — `task_iter.bpf.c`, installed with
  `bpftool iter pin`.
- **BPF arena** (Chapter 56) — `arena_list.bpf.c`, compiled with Clang and
  loaded via `bpftool`.

A few more chapters (the ring-buffer dynptr, timers, and CO-RE bindings —
Chapters 50, 54, 58) ship a working Aya kernel program *and* a canonical C
reference, because the Aya rendering is still approximate or depends on
generated bindings; each says so in its verification note. The point isn't that
Aya is lacking — it's that the goal is all-Rust while the reality in 2026 is
that the newest kernel surface still speaks C first, so the toolchain carries
both.

## Install cargo-generate and scaffold from the Aya template

Aya projects are scaffolded with `cargo-generate` from the official
`aya-template`. Install the generator:

```bash
cargo install cargo-generate
```

You'll create real projects per-chapter, but try the scaffold now to
see the shape:

```bash
cargo generate --name hello https://github.com/aya-rs/aya-template
```

`cargo-generate` asks a couple of questions (program type, whether to
use `#[no_std]` user space, etc.). The result is a Cargo **workspace**
with this layout:

```text
hello/
├── Cargo.toml            ← workspace
├── rust-toolchain.toml   ← pins the toolchain(s)
├── hello/                ← user-space crate (loads + attaches + reports)
│   └── src/main.rs
├── hello-ebpf/           ← kernel crate (the actual BPF program)
│   └── src/main.rs
├── hello-common/         ← types shared between the two halves
└── xtask/                ← build glue: compiles -ebpf, then -hello
```

The split is the whole Aya idea in one picture: `hello-ebpf` becomes
the in-kernel object, `hello` is the user-space loader/reporter, and
`hello-common` holds the `#[repr(C)]` structs both sides agree on so a
map entry written by the kernel deserializes correctly in user space.

Build it (this exercises nightly + `bpf-linker` + the xtask):

```bash
cd hello && cargo build
```

The first build is slow — it compiles `core` for the BPF target and a
chunk of the Aya crates. Subsequent builds are quick. If this
succeeds, your toolchain is complete. Chapter 6 explains every line of
what got generated and actually runs it against the target VM.

> **The Aya crate versions you'll see:** the user-space crate is `aya`
> (0.14.x line), the kernel crate is `aya-ebpf` (0.2.x line), logging
> is `aya-log` (user) + `aya-log-ebpf` (kernel). The template pins
> compatible versions; don't mix major lines by hand.

## RustRover setup

You have RustRover locally, which is a comfortable way to work with
the two-crate workspace.

1. **Open the workspace root** (the `hello/` directory with the
   workspace `Cargo.toml`), not an individual crate. RustRover reads
   the workspace and indexes all members.
2. **Let it pick up `rust-toolchain.toml`.** RustRover honors the
   pinned toolchain per crate, so `hello-ebpf` resolves against
   nightly while `hello` resolves against 1.96.0. If RustRover prompts
   to "attach" `Cargo.toml`, attach the workspace one.
3. **Expect red squiggles in `hello-ebpf` to be mostly cosmetic.** The
   BPF target is `no_std` and uses target-specific intrinsics the
   analyzer doesn't always model; if `cargo build` is green, trust the
   build over the inline analysis. You can point RustRover's external
   linter at `cargo clippy` for the user-space crate where analysis is
   reliable.
4. **Run/deploy from the terminal, not the IDE Run button.** Loading
   eBPF needs privileges *on the target VM*, not on your laptop. The
   per-chapter `demo.sh` builds in RustRover-friendly ways and then
   ships the binary to the guest with the `deploy-to-target.sh` from
   Chapter 2. Use RustRover to edit and to run `cargo build`/`clippy`;
   use the terminal to deploy.

### Step-debugging the loader on the VM (RustRover / gdb)

Editing locally and tailing logs gets you far, but sometimes you want to
**stop the loader on a line and look around** — inspect the `Ebpf` object
after `load()`, watch a ring-buffer read, see why an `attach()` returned
an error. You can do that against the guest, with one hard boundary to
keep straight first.

#### What is (and isn't) debuggable

The loader is an ordinary user-space Rust program — breakpoints, stepping,
variable inspection all work normally. But it runs **on the guest, under
`sudo`** (loading eBPF needs `CAP_BPF`/`CAP_SYS_ADMIN`), so the debugger
has to attach *there*, not on your laptop. The way to bridge that is
`gdbserver` on the VM and a `gdb` (or RustRover) that connects to it.

The **eBPF half is not source-line debuggable this way**. Once a program
is verified and loaded it runs in kernel context; there's no gdb stepping
into it. You "debug" the kernel side by *observing* it — `bpftool prog
dump xlated`, `bpftool map dump`, `aya-log`, and `bpftrace` (Chapter 5).
So: gdb for the loader, `bpftool`/`bpftrace` for the program.

#### The mechanism, from the terminal

RustRover's remote-debug run config is a GUI wrapper over this exact flow,
so it's worth doing once by hand — then the IDE config is obvious.

**1. Build with debug info.** The `dev` profile already carries it
(`cargo build` → `target/debug/<name>`, unoptimized + debuginfo). A
release binary is stripped of much of it; if you must debug an optimized
build, add `-C debuginfo=2`. Keep the binary **with symbols on the
laptop** — gdb reads symbols locally and only the *process* runs remote.

**2. Ship the binary and start `gdbserver` on the guest** (under `sudo`,
so the inferior has the caps to load eBPF):

```bash
[laptop]$ scp target/debug/hello fedora@<vm-ip>:/home/fedora/hello
[vm]$     sudo gdbserver 0.0.0.0:2345 /home/fedora/hello
          Process /home/fedora/hello created; pid = ...
          Listening on port 2345
```

(`gdbserver` ships in Fedora's `gdb-gdbserver` package.) `gdbserver`
launches the program stopped, waiting for a debugger.

**3. Connect from the laptop, pointing gdb at the local symbol-ful
binary:**

```bash
[laptop]$ gdb -q target/debug/hello
(gdb) target remote <vm-ip>:2345
(gdb) break main.rs:57          # e.g. the program.load() line
(gdb) continue
Thread 1 "hello" hit Breakpoint 1, hello::main::{async_block#0} () at hello/src/main.rs:57
57          program.load()?;
(gdb) bt        # full source backtrace, through the tokio runtime
```

That's the whole loop: a source-line breakpoint in the loader stops the
process running on the VM, with locals and backtrace resolving against
your local source. Two practical notes gdb will remind you of:

- **`set sysroot /`** (or just launching gdb with the local binary path,
  as above) keeps it from copying shared libraries over the wire — "file
  transfers from remote targets can be slow" otherwise.
- gdb may offer **debuginfod** auto-download on connect; fine to decline
  (`set debuginfod enabled off`) — you have the symbols you need locally.

#### Wiring it into RustRover

With the mechanism understood, the IDE config is a thin layer:

- **Add the guest as an SSH host** — *Settings → Tools → SSH
  Configurations*: host `<vm-ip>`, user `fedora`, the key from Chapter 2.
  (RustRover *Remote Development* — running the whole IDE backend on the
  guest — also works, but it's heavier than you need; SSH + remote debug
  is enough.)
- **A "Remote Debug" run configuration** (GDB Remote Debug): *Debugger*
  = "Attach to remote GDB server", target `<vm-ip>:2345`, *Symbol file*
  = your local `target/debug/<name>`, *Sysroot* = `/`. Set breakpoints in
  the loader source and hit Debug; RustRover speaks the same gdb remote
  protocol you just used.
- **A "Before launch" step** that runs `scripts/lab/deploy-to-target.sh`
  (Chapter 2) — or a small wrapper that also `ssh`es in to start
  `sudo gdbserver … <binary>` — so *build → ship → run-under-debugger* is
  one green button.

#### Caveats

- **`sudo` on the guest.** The inferior must have `CAP_BPF`; run
  `gdbserver` under `sudo` (the lab's `fedora` user has passwordless
  sudo). Attaching to an already-running loader instead? `sudo gdbserver
  --attach :2345 <pid>`.
- **Binary paths must match** between the `scp` destination, the
  `gdbserver` argument, and RustRover's symbol file — mismatched builds
  give "breakpoint set but not yet resolved" and never stop.
- **eBPF ≠ steppable.** Breakpoints only bind in the loader. To "watch"
  the program, inspect its maps/ring buffers and `aya-log` output — the
  kernel-side visibility tools are Chapter 5, not gdb.

## The eBPF tooling landscape

A quick map of the tools you'll meet, because the ecosystem grew in layers and
the names blur together:

- **`bpftool`** — the swiss-army knife, and the one you'll reach for constantly.
  It lists and inspects loaded programs and maps (`prog show`, `map dump`), dumps
  BTF (`btf dump`), pins and registers things (`struct_ops register`, `iter
  pin`), probes kernel features (`feature probe`), and generates skeletons. When
  a chapter cross-checks Aya's behaviour against ground truth, it's almost always
  `bpftool`.
- **libbpf** — the canonical C loader library: it loads objects, performs the
  CO-RE relocations (Chapter 58), and manages maps and links. Aya is its
  pure-Rust counterpart and does *not* use it, but the kernel docs and the
  reference `.bpf.c` programs assume it.
- **BCC** — the original framework: it bundles Clang and compiles eBPF *on the
  target at runtime*. Powerful but heavy (the very problem CO-RE later solved);
  its Python tools (`opensnoop`, `execsnoop`) are echoed in this book's early
  chapters, reimplemented in Aya.
- **bpftrace** — a high-level tracing DSL on the same machinery, for one-liners
  and quick scripts where a full program is overkill.
- **Clang / LLVM** — the compiler backend underneath all of it, as above.
- **pahole / `dwarves`** — turns DWARF debug info into BTF and shows struct
  layouts; it's how a kernel gains the BTF that CO-RE and `vmlinux.h` depend on.
- **`llvm-objdump` / `readelf`** — inspect the compiled object, including the
  CO-RE relocation records (Chapter 58's cross-check).
- **`perf`** — sampling and tracing that predates and complements eBPF, useful
  for profiling alongside it (Chapter 23).

The split that matters operationally: **you build on the laptop** (rustc,
bpf-linker, cargo, and Clang for the C references) and **you inspect on the
guest** (`bpftool`, `bpftrace`, `bcc-tools`, `perf`) — because those read the
kernel where the program actually runs, which is why Chapter 2's cloud-init put
them in the VM. As a project policy these always come from Fedora/Red Hat
repositories, never third-party binaries.

## The loader's libc: glibc vs musl

Your eBPF object is special — verified bytecode for the kernel VM — but the
**loader is an ordinary Linux userspace binary**, and like any such binary it
links a C library for its syscalls, memory allocation, and DNS. *Which* libc,
and whether it's linked statically or dynamically, decides where that binary can
run — a portability axis entirely **separate** from the kernel-version
portability CO-RE gives you (Chapter 58). The two get conflated constantly; keep
them apart.

- **glibc** — the GNU C Library, the default on Fedora, RHEL, the UBI images
  this book uses, Debian, and nearly every mainstream distro. Rust's default
  Linux target, `x86_64-unknown-linux-gnu`, links it, normally **dynamically**:
  the binary carries a dependency on the host's glibc. glibc is forward
  compatible (build against an older glibc, run on a newer one), so a binary
  built on Fedora 44 runs on Fedora 44 and later without fuss — but drop it on a
  distro with an *older* or absent glibc and it may refuse to start.
- **musl** — a small, clean libc built for static linking. It is **Alpine
  Linux's** default (a frequent confusion — Arch, despite the similar-sounding
  name, uses glibc). On Fedora it isn't the system libc at all; it's a **Rust
  build target you opt into**, `x86_64-unknown-linux-musl`, which by default
  produces a **fully static** binary with *no* dynamic libc dependency.

The practical upshot:

- A **dynamic glibc** loader is the path of least resistance and exactly right
  for this lab: we build on Fedora and `scp` the binary to Fedora 44 targets we
  control, so the glibc versions line up by construction. That's what every
  `demo.sh` here produces, and it's all a homogeneous fleet needs.
- A **static musl** loader is what you build when *one* artifact must run across
  unlike distributions — drop the same file on Alpine, Debian, and Fedora and it
  just runs, because it depends on nothing outside the kernel ABI. This is the
  deployment story Aya advertises, and it pairs naturally with CO-RE: musl static
  fixes *userspace* portability, CO-RE fixes *kernel* portability, and together
  they give a single self-contained binary for a heterogeneous fleet.

If you ever want the musl build, it's a target away:

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

(On Fedora the musl target usually links self-contained, though some setups want
`musl-gcc` from `sudo dnf install musl-gcc`.) The trade-offs are real — musl's
allocator and DNS resolver behave differently from glibc's, and a few glibc-only
features are absent — so reach for it when portability demands it, not by
default. For everything in this book, the stock glibc target is correct.

## What you should have now

- [x] `rustup` installed; Rust 1.96.0 (or 1.95.0 with the timing note)
  as default
- [x] `nightly` toolchain with `rust-src`
- [x] `bpf-linker` and `cargo-generate` installed
- [x] A scaffolded `hello` workspace that **builds**
- [x] RustRover opened on the workspace, toolchains resolving
- [x] A mental model of the compiler path (rustc + bpf-linker for Aya;
  Clang for the C references and `vmlinux.h`) and the inspect-on-the-guest
  tooling (`bpftool` et al.)
- [x] Awareness of the **glibc** (our default) vs **musl** (optional, static,
  cross-distro) choice for the loader — distinct from CO-RE's kernel portability

[Next: Chapter 5 — eBPF concepts and tools →]({{ "/docs/05-ebpf-concepts/" | relative_url }})

---

*Verification status: <span class="status status--verified">verified — Fedora 44 host</span>.
This toolchain built the entire corpus (~60 examples) on this Fedora host
throughout the smoke campaign: the nightly + `rust-src`/`build-std` path for the
kernel crate, `bpf-linker` (including the LLVM shared-lib fallback), the stable
loader toolchain, and the per-crate `rust-toolchain.toml` pins all work as
described. The gdbserver remote-debug path (RustRover / gdb) was exercised
end-to-end. Aya crate versions: aya 0.14.x, aya-ebpf 0.2.x.*
