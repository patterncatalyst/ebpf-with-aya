---
title: "The Rust + Aya toolchain"
order: 4
part: Foundations
description: Install Rust 1.96.0 via rustup (not dnf), add the nightly BPF target and bpf-linker, scaffold projects with cargo-generate and the aya-template, and set up RustRover.
duration: 25 minutes
---

This chapter installs everything needed to *build* eBPF programs in
Rust on your laptop. Aya is unusual and pleasant here: the kernel-side
and user-side are both Rust, there's no C toolchain in the loop, and a
release build produces one self-contained binary you ship to the
target VM. But the eBPF half compiles to a special target with a
special linker, so the setup has a few moving parts. We'll get them
all in place and prove them with a build in Chapter 6.

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
> (0.13.x line), the kernel crate is `aya-ebpf` (0.1.x line), logging
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

## Kernel-side tooling lives in the VM

You already installed `bpftool`, `bpftrace`, `bcc-tools`, and `perf`
*in the target VM* via cloud-init (Chapter 2). That's deliberate:
those tools inspect the kernel where the program runs. You build on
the laptop; you inspect on the guest. As a project policy these always
come from Fedora/Red Hat repositories — never third-party binaries.

## What you should have now

- [x] `rustup` installed; Rust 1.96.0 (or 1.95.0 with the timing note)
  as default
- [x] `nightly` toolchain with `rust-src`
- [x] `bpf-linker` and `cargo-generate` installed
- [x] A scaffolded `hello` workspace that **builds**
- [x] RustRover opened on the workspace, toolchains resolving

[Next: Chapter 5 — eBPF concepts and tools →]({{ "/docs/05-ebpf-concepts/" | relative_url }})

---

*Verification status: <span class="status status--unverified">unverified</span>.
Toolchain steps, the `1.96.0` pin, and the `bpf-linker` LLVM fallback
have not yet been confirmed on a clean Fedora 44 laptop. The Aya crate
version lines were checked against crates.io at authoring (aya 0.13.x,
aya-ebpf 0.1.x).*
