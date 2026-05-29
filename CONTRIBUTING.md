# Contributing

Short version: this project uses **Conventional Commits**, ships in
**iterations** named `ebpf-with-aya-rNN.x.tar.gz`, marks every claim
**`unverified`** until it's run on real Fedora 44 hardware, and holds
two hard policies — **source provenance** and **tooling provenance** —
described below.

## Commit-message format

```
<type>(<scope>): <short summary>

<optional body, wrap at 72 chars>

<optional trailers, e.g. Fixes: #123>
```

- `<type>` from the table below.
- `<scope>` is optional but expected on `docs:` and `demo:` commits.
  Use `ch00`…`chNN` for chapter work matching `_docs/NN-*.md`,
  `demo-NN` for example work matching `examples/NN-*/`, or omit when
  the change spans many areas.
- `<short summary>` is one line, imperative mood, ≤ 72 chars, no
  trailing period.

### Types

| Type | When to use |
|------|-------------|
| `docs:` | Tutorial prose under `_docs/`, README, PRD, plan updates |
| `site:` | Jekyll layouts, includes, CSS, page structure |
| `demo:` | Anything inside `examples/NN-*/` (Aya code, manifests, demo.sh) |
| `ci:` | `.github/workflows/`, helper/test scripts under `scripts/` |
| `chore:` | Routine maintenance, dependency bumps, `.gitignore`, iteration archive housekeeping |
| `fix:` | Bug fix in any of the above; always pair with the scope |
| `feat:` | New capability; always pair with the scope |
| `refactor:` | Reorganization without behaviour change |
| `style:` | Formatting only, no logic change |

### Examples

```
docs(ch06): explain the per-CPU counter aggregation in hello-world
demo-06: add bpftool cross-check to the hello-world demo.sh
site: re-theme accent from Red Hat red to eBPF amber
chore: archive r01 — scaffold + Foundations chapters 0-6
feat(demo-27): tcpconnlat XDP program with OTLP latency histogram
```

## Iteration cadence

This project ships in **iterations** named `rNN` with optional
sub-iterations `rNN.x` (or `rNNa`, `rNNb`). Each iteration:

1. Drops as a tarball: **`ebpf-with-aya-rNN.x.tar.gz`** (this is r1.0).
2. Extracts **in place** over the working copy — it never overlays old
   files into stale locations; review with `git diff --stat` first.
3. Gets committed with `chore: archive rNN — <summary>`.
4. Pushes to `main`; `gh run watch` confirms the Pages deploy.

The naming convention is deliberate: `rNN` is the iteration, `rNN.x`
is a sub-iteration, so you never continually overlay older files. The
roadmap in [`_plans/iteration-plan.md`](./_plans/iteration-plan.md)
maps every tutorial topic to its iteration.

### The verification loop

Per [`onboarding/LESSONS-LEARNED.md`](./onboarding/LESSONS-LEARNED.md):
ship **tested code first, then prose**. For any iteration that ships an
Aya program:

1. Tarball delivered with example dir, chapter prose, and
   reconciliation rows marked `unverified` / `in flight`.
2. Extract, push, `gh run watch` confirms the site build is green.
3. `cd examples/NN-name/ && ./demo.sh`; share output.
4. **Pass** → next iteration's first move is flipping that row to
   `verified (Fedora 44)`. **Fail** → diagnose from output, fix in a
   sub-iteration (`rNNa`), re-run.

**Claude (or any AI assistant) must not self-promote a claim to
`verified`.** Promotion requires a human running the test on the target
and recording the result.

## Source-provenance policy

This project takes **insight** from the entire global eBPF community —
papers, talks, blog posts, and the design of well-known projects are
all fair game to learn from and to cite. What we ship, though, is
**our own code**: we don't copy, vendor, or port code line-for-line
from other repositories.

- *Insight* (reading a writeup, understanding an approach, citing a
  finding) — encouraged.
- *Copying code* (snippets, vendoring, line-for-line ports) — no; write
  an original equivalent instead.

Anything we do borrow must carry a clearly compatible license, and the
borrowed approach should be noted in the chapter's prose or the
reconciliation plan. When a source's licensing or origin is unclear,
don't use its code — find a clearly-licensed source or write it
ourselves.

## Tooling-provenance policy (hard rule)

All kernel and eBPF tooling — `bpftool`, `bpftrace`, `bcc`/`bcc-tools`,
`perf`, `clang`/`llvm`, `kernel-devel` — is installed **only from
Fedora and/or Red Hat package repositories** via `dnf`. No upstream
release binaries, no `curl | sh` installers, no third-party COPRs for
these tools. The cloud-init in `scripts/lab/cloud-init/` and every
chapter follow this.

The two exceptions, which are Rust-ecosystem build tooling rather than
kernel tooling, are installed via `rustup`/`cargo` as documented in
Chapter 4: the Rust toolchain itself (`rustup.rs`), `bpf-linker`, and
`cargo-generate`. `bpf-linker`'s LLVM dependency, when needed, comes
from Fedora's `llvm`/`llvm-devel`.

## Container image policy

**Everything that runs in user space runs in a container** — clients,
load drivers, and the language/application targets we observe — via
**Podman** or **podman-compose**. The *one* exception is the Aya
user-space loader for an eBPF program: it runs as a privileged binary
**on the target VM** (it must load into that kernel) and is deployed by
`scp` + `sudo`, not containerized.

Container build rules:

- **UBI base images only**, fully qualified
  (`registry.access.redhat.com/ubi9/...`; the observability stack uses
  the fully-qualified `docker.io/grafana/otel-lgtm`). Bare short names
  don't resolve under Fedora's registry policy. Document any non-UBI
  exception inline and here.
- **Multi-stage Containerfiles**, always: a builder stage (compile /
  install deps / produce artifacts) and a slim runtime stage that copies
  only what's needed. This keeps images small and keeps build tooling
  out of the runtime image.
- Bind mounts carry **`:Z`**; published ports bind **`127.0.0.1`**;
  named volumes are root-owned under rootless Podman (use `tmpfs` +
  `user: "0"` where a service needs a writable dir, as the stack does).

### Container runtime: crun, eBPF, and SELinux

Fedora's default OCI runtime is **crun** (pinned at **1.27.1** for this
tutorial). It matters here for two reasons:

- **Observing a containerized target** means the process you probe lives
  in its own PID and mount namespaces. uprobes attach to the target
  binary at its path **on the host** (the overlay merged/upper dir) or
  by PID; the probe sees **host** PIDs, so user space may need to
  translate to the in-container PID. The chapters that observe Quarkus
  and FastAPI targets spell this out.
- crun runs under **SELinux** confinement and supports cgroup-v2 /
  eBPF-based controllers; running our own eBPF tooling alongside
  containerized targets has to respect both. Where a demo needs extra
  capabilities or an SELinux label, it's called out in that example's
  README.

## Language & application targets

Observed application targets are pinned to:

- **Java 25** (LTS) with **Quarkus LTS 3.33**, containerized (UBI +
  multi-stage).
- **Python 3.14** with **FastAPI**, containerized (UBI + multi-stage).

Client load drivers are **Python 3.14** in Podman. These are the
*targets/clients*; the eBPF programs themselves remain Rust/Aya.

## Diagrams

Architecture and data-flow diagrams are authored in **Excalidraw** and
committed as a pair under `assets/diagrams/`: an editable
`<name>.excalidraw` source and an exported `<name>.svg`. Embed them with
the `excalidraw.html` include (see `assets/diagrams/README.md`). Prefer
a diagram over a long prose description for anything spatial — lab
topology, data paths, attach/lifecycle.

## Site authoring conventions (Liquid collisions)

Jekyll's Liquid uses `{% raw %}{{ }}{% endraw %}` and
`{% raw %}{% %}{% endraw %}`. eBPF/Rust content rarely collides (Rust
format strings use single braces), but Grafana templating, Go
templates, and some config snippets do.

- **`_docs/*.md`** — wrap any code block or inline span containing
  literal `{% raw %}{{ }}{% endraw %}` in `{% raw %}` / `{% endraw %}`.
  Never wrap an active `relative_url` image src.
- **`_plans/*.md`** — set `render_with_liquid: false` in front matter
  (these files reference templating syntax in tables).
- Use `[placeholder]` not `<placeholder>` in inline backticks; kramdown
  reads `<...>` as an HTML tag and can swallow content.
- Jekyll permalink slugs include the `NN-` prefix
  (`/docs/06-hello-world/`, not `/docs/hello-world/`).

## Branching and PRs

- Default branch: `main`. Branches: `feat/<thing>`, `fix/<thing>`,
  `docs/<scope>`.
- One commit per logical change preferred; squash-merge is fine.
- Force-pushing your own feature branch is fine; `main` is not.

## Reconciliation plan

Every substantive change leaves a corresponding entry in
[`_plans/reconciliation-plan.md`](./_plans/reconciliation-plan.md). It
tracks **verification state, not commits**. A change you've run
end-to-end on Fedora 44 → flip the row to `verified (Fedora 44)`. A
change you haven't run → leave it `unverified` and say so. Keep it
honest; it's the difference between documentation and confident
fiction.
