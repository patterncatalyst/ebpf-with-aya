---
render_with_liquid: false
---
# Backlog / TODO

## Tooling-coverage pass (requested r42) — ensure setup installs everything the book uses
Audit every CLI/tool referenced across all chapters and confirm each is (a) installed
on the correct host by the Chapter 2 cloud-init / setup scripts (laptop vs ebpf-target
VM), and (b) documented on the foundation pages (Ch 1 prerequisites, Ch 2 lab-setup,
Ch 4 toolchain). Then add a **tooling table** in the foundational setup mapping:
tool → where it runs → install source (Fedora repo pkg / rustup / cargo) → usage/value-prop.

Known tools to verify are covered (non-exhaustive — grep the chapters/examples for more):
- **Kernel/eBPF**: bpftool, bpftrace, bcc-tools, perf, dwarves (pahole), libbpf-devel,
  clang, llvm (llvm-objdump, llvm-objcopy), elfutils.
- **Power/Ch61**: turbostat, powerstat (likely from `kernel-tools`/`cpupower`/`turbostat` pkg) — CONFIRM the package name on Fedora 44 and add to VM setup.
- **Net/inspect**: ss, ip (iproute2), tcpdump, ethtool, libxdp/xdp-tools (xdpdump), nstat.
- **Rust/build**: rustup, cargo, bpf-linker, cargo-generate, aya-tool, bindgen (libclang),
  musl target + musl-gcc (Ch4 optional), cross (if used).
- **Containers/clients**: podman, podman-compose, buildah, skopeo.
- **Misc used in demos**: openssl, jq, curl, scp/ssh, git, gh (GitHub CLI), make.
- **Per-language targets**: Python 3.14 (UBI), Java 25 + Quarkus (UBI/maven), FastAPI.

Deliverable: update Ch 2 cloud-init package list + a "Tooling reference" table in the
foundation (Ch 2 or Ch 4). Flag any tool a chapter uses that the setup never installs.

## (add future backlog items below)
