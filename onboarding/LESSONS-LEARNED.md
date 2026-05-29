# Lessons learned

Empirical guidance for this project, in three areas: **the lab
(Podman, KVM, Fedora)**, **Jekyll on GitHub Pages**, and **working
with an AI assistant on a long technical build**. Opinions here are
practical, not theoretical. Some carry over from the minikube/Hummingbird
tutorials this project's structure is modeled on; the eBPF/Aya/KVM
specifics are new.

---

## Part 1 — The lab: Podman, KVM, Fedora

### Deploy eBPF to a VM, never the host kernel
A bad XDP, LSM, or `sched_ext` program can wedge the kernel it loads
into. On a work laptop that's a reboot and lost state; on a disposable
KVM guest it's a `destroy-vm.sh && provision-vm.sh`. Every chapter
builds on the host and deploys the artifact to `ebpf-target`. The host
kernel is never the target.

### Build once, ship the artifact
Aya produces a single self-contained user-space binary with the BPF
object embedded. Build on the laptop (fast CPU, warm caches, RustRover),
`scp` the binary to the guest, run under `sudo`. The guest needs only a
kernel — no Rust toolchain. This is the whole "compile once, run
everywhere" story that musl + CO-RE buys you.

### Confirm BTF on the guest before trusting CO-RE
CO-RE relocations need `/sys/kernel/btf/vmlinux`. Fedora 44's stock
cloud kernel ships it; cloud-init checks and logs it. If it's missing
you're on an unusual kernel — switch to stock before debugging anything
else.

### Use Podman, fully-qualified images, `:Z`, `127.0.0.1`
- Podman is the Fedora default and rootless by default.
- Image names must be fully qualified (`docker.io/grafana/otel-lgtm`),
  the bare short name doesn't resolve under Fedora's registry policy.
- Bind mounts need `:Z` for SELinux; it's a harmless no-op elsewhere.
- Use `127.0.0.1`, never `localhost` — `localhost` may resolve to IPv6
  `::1` while a runtime binds IPv4 only.
- A rootless container reaches a host port via
  `host.containers.internal`.

### Rootless podman + named volumes are root-owned
Grafana in `otel-lgtm` runs as `user: "0"` with `tmpfs` for `/tmp`
rather than a named data volume, to sidestep root-owned-volume
permission errors under rootless podman.

### Kernel tooling comes from Fedora repos only
`bpftool`, `bpftrace`, `bcc-tools`, `perf`, `clang`, `llvm`,
`kernel-devel` — all via `dnf`, never upstream binaries or `curl | sh`.
This is a hard project policy (see `CONTRIBUTING.md`). The Rust-side
build tools (`rustup`, `bpf-linker`, `cargo-generate`) are the
documented exception.

### bpftool and bpftrace are your ground truth
When your Aya user space reads zeros, `bpftool map dump` tells you
whether the *kernel* side wrote anything — isolating the bug to one
half. `bpftrace -e '...'` gives an independent count to compare against
what your program reports. Always cross-check; never trust your own
reporting alone.

---

## Part 2 — Jekyll and GitHub Pages

### Deploy via the Actions workflow, not built-in Pages Jekyll
`.github/workflows/pages.yml` runs the Gemfile-pinned Jekyll 4.x, not
GitHub's old built-in 3.10. Set Settings → Pages → Source: GitHub
Actions.

### Baseurl is the most common deploy bug
Project Pages (`USER.github.io/REPO`) need `baseurl: "/REPO"` — here
`/ebpf-with-aya`. Local dev overrides with `--baseurl ""`. Internal
links use the `relative_url` filter, never hard-coded paths.

### Liquid collisions
- `_docs/*.md`: wrap any literal `{{ }}` (Grafana templating, Go
  templates) in `{% raw %}`/`{% endraw %}`. Rust format strings use
  single braces and are fine. Never wrap an active `relative_url`
  image src.
- `_plans/*.md`: set `render_with_liquid: false` in front matter.
- Use `[placeholder]` not `<placeholder>` in inline backticks; kramdown
  reads `<...>` as HTML.
- Dashboard JSON and other `{{ }}`-heavy content lives under
  `examples/`, which is excluded from the build — no collision there.

### Hand-rolled CSS, kramdown + rouge
`assets/css/site.css` is hand-rolled (Red Hat fonts, eBPF amber accent),
no build step. kramdown with GFM + rouge gives tables, fenced code with
language hints, and syntax highlighting — everything a tutorial needs.

### Permalink slugs include the NN- prefix
`_docs/06-hello-world.md` → `/docs/06-hello-world/`. Rename a file and
you must update every link to it.

---

## Part 3 — Working with an AI assistant on a long build

### The reconciliation plan is the most important file
An AI will produce plausible-looking claims that may not match reality.
The reconciliation plan tracks every claim's verification state. New
claims default to `unverified`. They are promoted to
`verified (Fedora 44)` only when a human runs the test and sees it pass.
**The AI must not self-promote.** This is non-negotiable.

### Tested code first, then prose
Get the Aya program building and running, fix the bugs, *then* write the
chapter to match. Prose-first encourages confident fiction because the
prose has to assert something. r1.0 is an explicit exception — it ships
the lab that verification runs against, so its code is honestly marked
unverified pending the first real run.

### Aya/OTel APIs churn; treat the first build as the test
The `aya-template` has moved between an `xtask` build and the
`aya-build` `build.rs` approach; the `opentelemetry` Rust crate's
exporter builders move between minor versions. When a shipped example
doesn't compile, generate a fresh scaffold for the *installed* versions
and reconcile. Record the fix in the reconciliation plan.

### Ship tarballs, paste single-line commands
Deliver iterations as `ebpf-with-aya-rNN.x.tar.gz` via the file tool.
Long inline pastes get mangled (auto-linkified URLs, reformatted YAML,
broken multi-line commands in zsh). Multi-step procedures ship as
scripts; pasted commands are single-line.

### The test-on-real-hardware loop
1. AI proposes code/commands. 2. Human runs them on Fedora 44.
3. Human pastes output (success or error) back. 4. AI debugs from the
actual error, not what it "should" be. Slow, but it produces code that
works instead of code that looks like it works.

### Recap at phase boundaries
Long sessions drift. At each phase boundary, summarize what's verified,
what's pending, and what was decided. The reconciliation plan's
Section D is where that lands.

### Don't continually overlay old files
The `rNN`/`rNN.x` tarball naming exists so iterations extract in place
without leaving stale copies of renamed/moved files around. Review with
`git diff --stat` before committing each iteration.

---

## TL;DR

1. Deploy eBPF to the VM; build on the host; ship the artifact.
2. Podman, fully-qualified images, `:Z`, `127.0.0.1`,
   `host.containers.internal`.
3. Kernel tooling from Fedora repos only; cross-check with
   bpftool/bpftrace.
4. Pin Jekyll, deploy via Actions, baseurl `/ebpf-with-aya`.
5. Reconciliation plan from day one; new claims `unverified`; AI never
   self-promotes.
6. Tested code first; ship tarballs; single-line pastes; test on real
   hardware.
