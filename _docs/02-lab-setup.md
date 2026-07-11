---
title: "Lab setup: the eBPF target VM(s)"
order: 2
part: Foundations
description: Provision a disposable Fedora 44 guest under libvirt with one script, prove you can deploy a binary to it, and add a second guest plus the network between them for two-host tests.
duration: 30 minutes
---

This chapter builds the place your eBPF programs actually run. You
provision a Fedora 44 guest from a single script, confirm it has the
kernel BTF and tooling eBPF needs, and rehearse the deploy loop —
copy a binary in, run it under `sudo`, watch output come back — that
every later chapter uses. Then you add an optional second guest and
wire up the network between the two for the chapters that need real
host-to-host traffic.

All the scripts referenced here ship in the iteration tarball under
`scripts/lab/`. Run them from that directory.

<div class="callout">
  <p class="callout__title">Reading the command blocks</p>
  <p>From here on, a prefix on the prompt tells you <em>where</em> to run a
  command, and angle brackets mark a value you substitute:</p>
  <ul>
    <li><code>[host]$</code> — your laptop / dev machine, where you build.
    (Unprefixed commands and the <code>scripts/lab</code> scripts also run
    here.)</li>
    <li><code>[vm]$</code> — inside the <code>ebpf-target</code> guest (over
    SSH). This is the single guest used by most chapters.</li>
    <li><code>[peer]$</code> — inside the second guest,
    <code>ebpf-peer</code> (networking chapters only).</li>
    <li><code>&lt;iface&gt;</code>, <code>&lt;target-ip&gt;</code>,
    <code>&lt;pid&gt;</code>, … — a placeholder: replace it (brackets and
    all) with your real value. <code>&lt;iface&gt;</code> is the target's
    network interface, almost always <code>enp1s0</code> — see
    <a href="#finding-the-interface-to-attach-to">Finding the interface</a>
    below.</li>
  </ul>
  <p>To get a shell on a guest: <code>[host]$ ssh fedora@$(scripts/lab/vm-ip.sh
  ebpf-target)</code>. The example <code>demo.sh</code> scripts detect the IP
  and interface for you; you only substitute placeholders when running
  something by hand.</p>
  <p><strong>Credentials:</strong> there is no password to remember. The
  guest user is <code>fedora</code>, login is <em>SSH-key only</em> (the key
  you create in the next section — the account password is locked), and
  <code>sudo</code> is passwordless, which is why every <code>[vm]$ sudo
  …</code> in these chapters runs without prompting. This is a throwaway lab
  VM reachable only from your host on the libvirt network; if you ever need a
  console password, set one with <code>[vm]$ sudo passwd fedora</code>.</p>
</div>

{% include excalidraw.html
   file="lab-topology"
   alt="The lab: a Fedora 44 host laptop running the Aya build plus Podman containers (the otel-lgtm stack, Python clients, and Java/Python app targets), a target KVM VM where eBPF loads and attaches, and an optional peer KVM VM for two-host networking tests. The host scp's the built binary to the target and runs it under sudo; the target exports OTLP back to the stack; the target and peer exchange test traffic over the libvirt network."
   caption="Figure 2.1 — The lab at a glance: what runs where" %}

## One SSH key for the lab

The provisioning script injects your SSH public key into the guest so
you get passwordless login. If you don't already have an ed25519 key:

```bash
test -f ~/.ssh/id_ed25519.pub || ssh-keygen -t ed25519 -N "" -f ~/.ssh/id_ed25519
```

## Provision the target VM

The whole guest is one command. From `scripts/lab/`:

```bash
cd scripts/lab && ./provision-vm.sh ebpf-target
```

`provision-vm.sh` does five things, all visible in its output: caches
the Fedora 44 Cloud Base `qcow2` once under `~/.cache/ebpf-with-aya/`,
creates a thin per-VM overlay disk on top of it (so rebuilds are
cheap and the base stays pristine), renders a cloud-init seed with
your hostname and SSH key, and boots the domain with `virt-install
--import` attached to libvirt's `default` NAT network.

> **One thing to verify before first run** — Fedora rotates the exact
> Cloud Base filename each release (the `-1.5` build suffix changes).
> Open the `BASE_URL` printed by the script in a browser, copy the
> real `Fedora-Cloud-Base-Generic-44-*.x86_64.qcow2` name, and set it
> as `BASE_IMG` at the top of `provision-vm.sh`. This is the one
> manual value the script can't guess for you.

cloud-init takes about a minute to install packages on first boot.
The seed (`scripts/lab/cloud-init/user-data.tmpl`) installs everything
the target needs — all from Fedora repositories — including
`kernel-devel`, `clang`/`llvm`, `bpftool`, `bpftrace`, `bcc-tools`,
and `perf`, and it writes a readiness marker once BTF is confirmed.

### What the lab's tooling is for

You won't touch most of these by hand for a while, but it's worth knowing what's
on the box and why — and which side of the lab each lives on. The **guest VM**
gets the kernel-side toolchain (installed by cloud-init above); the **laptop**
gets the Rust build chain (Chapter 4, via `rustup` — *not* `dnf`). All of it
comes from Fedora repositories or `rustup`, never third-party binaries.

| Tool | Package / source | Where | What it's for |
|---|---|---|---|
| `bpftool` | `bpftool` | VM | inspect/manage loaded programs, maps, links — the ground-truth cross-check (Ch 5, 65) |
| `bpftrace` | `bpftrace` | VM | quick one-liner tracing to confirm an event fires before writing Aya (Ch 5, 64) |
| `bcc-tools` | `bcc-tools` | VM | the ready-made `*snoop`/`*latency` tracers used as cross-checks (Ch 66) |
| `libbpf-tools` | `libbpf-tools` | VM | precompiled CO-RE builds of the bcc tools — no runtime clang/headers (Ch 66) |
| `perf` | `perf` | VM | sampling, PMU counters, `perf stat` energy/cycles cross-checks (Ch 23, 61) |
| `turbostat` | `kernel-tools` | VM | per-package power/frequency/idle stats — the RAPL cross-check (Ch 61) |
| `clang` / `llvm` | `clang`, `llvm` | VM | compile reference `.bpf.c`, generate `vmlinux.h`, run classic bcc (Ch 4, 55–58, 66) |
| `dwarves` (`pahole`) | `dwarves` | VM | inspect BTF / struct layout from DWARF (Ch 5, 15) |
| `kernel-devel` + BTF | `kernel-devel`, in-kernel | VM | kernel headers + `/sys/kernel/btf/vmlinux` for CO-RE relocation (Ch 5, 58) |
| `jq` | `jq` | VM | parse `bpftool -j` and Tempo JSON in demos/cross-checks (Ch 62, 65, 66) |
| `openssl` | `openssl` | VM | generate `traceparent` ids; a TLS target for `sslsniff` (Ch 14, 62, 63) |
| `podman` / `podman-compose` / `crun` | same | VM | run the observed Quarkus/FastAPI targets so the VM kernel sees them (Ch 16, 45–47, 63) |
| `iproute` (`ss`/`ip`), `tcpdump`, `nmap-ncat`, `socat` | same | VM | traffic generation + inspection for the networking part (Ch 27–36) |
| `rustup` → Rust 1.96.0, BPF target | `rustup` | laptop | compile your Aya programs (Chapter 4) — never installed via `dnf` |
| `bpf-linker`, `cargo-generate`, `aya-tool` | `cargo install` | laptop | link BPF objects, scaffold projects, generate `vmlinux` bindings (Ch 4) |

If a later chapter reaches for a tool that isn't here, it says so and how to
install it; but the set above covers everything through the capstone.

Find the guest's leased IP and connect:

```bash
./vm-ip.sh ebpf-target && ssh fedora@"$(./vm-ip.sh ebpf-target)"
```

If `vm-ip.sh` says "no lease yet", cloud-init is still booting — wait
twenty seconds and retry.

## Confirm the guest is eBPF-ready

eBPF with CO-RE (Chapter 5) needs the running kernel to expose its own
type information as **BTF** at `/sys/kernel/btf/vmlinux`. The cloud-init
run already checked this and logged the result. Confirm it yourself
inside the guest:

```bash
[vm]$ cat /var/log/ebpf-target-ready && ls -l /sys/kernel/btf/vmlinux && bpftool version
```

You want to see `BTF: present`, a `vmlinux` file of a few megabytes,
and a `bpftool` version string. A stock Fedora 44 cloud kernel ships
BTF by default, so this should just pass. If `BTF: MISSING` appears,
you are on an unusual kernel build — note it in the reconciliation
plan and switch to a stock Fedora kernel before continuing.

While you're in the guest, confirm it can load *any* BPF program at
all — `bpftool` listing programs is a harmless probe:

```bash
[vm]$ sudo bpftool prog list | head
```

An empty list (or a few system programs) printed without error means
the BPF syscall surface is available to you. Log out with `exit`.

## Rehearse the deploy loop

Every later chapter ends by shipping a compiled user-space binary to
the target and running it there. `deploy-to-target.sh` is that loop in
one command: it resolves the guest IP, `scp`s the binary in, and runs
it over SSH under `sudo` (eBPF loading needs `CAP_BPF` /
`CAP_SYS_ADMIN`, and the lab user has passwordless sudo).

You don't have an Aya binary yet — that's Chapter 6 — so rehearse with
something trivial that proves the path works end to end. Build a
two-line "binary" on the host and ship it:

```bash
printf '#!/usr/bin/env bash\necho "deploy loop OK on $(hostname), kernel $(uname -r)"\n' > /tmp/hello-deploy && chmod +x /tmp/hello-deploy
```

```bash
cd scripts/lab && ./deploy-to-target.sh ebpf-target /tmp/hello-deploy
```

You should see `deploy loop OK on ebpf-target, kernel 6.x…` printed
back from the guest. That is the exact mechanism Chapter 6 will use
with a real Aya program — only the binary changes.

> **Why deploy a binary instead of building on the guest?** Aya
> produces a single self-contained user-space binary that embeds the
> eBPF object. You build once on the laptop (fast CPU, RustRover,
> caches warm) and ship the artifact. The guest never needs the Rust
> toolchain — only a kernel. This is the same "build once, deploy the
> artifact" story that makes Aya's musl + CO-RE combination so
> portable, and it's covered properly in Chapter 4.

## Host-side gotchas that block the loop

Three host-configuration issues can silently break provisioning or the
telemetry path. They're environment, not code, so they're easy to miss:

- **qemu can't read the VM disk (provisioning fails).** `provision-vm.sh`
  stages the overlay disk under `~/.cache`, but on `qemu:///system` the
  hypervisor runs as the `qemu` user (uid 107) and can't traverse a `0700`
  home directory. If `virt-install` fails with
  `Cannot access storage file … (as uid:107): Permission denied`, grant the
  qemu user *search* on the path (not world-readable):
  ```bash
  sudo setfacl -m u:qemu:x "$HOME" "$HOME/.cache"
  ```
  (SELinux-enforcing hosts may also relabel; this lab host runs with SELinux
  disabled.)

- **`podman-compose` is a host tool.** The compose-based examples (03, 62, 63) run on the host, not the guest. Install it in your user environment: `pip install --user podman-compose`.

- **Use the system libvirt, not the session one.** VMs and the `default`
  network live on `qemu:///system`. If `virsh`/`virt-install` can't find the
  `default` network, set `export LIBVIRT_DEFAULT_URI=qemu:///system` (and make
  sure your user is in the `libvirt` group).

- **The guest can't reach the host stack (no telemetry).** Every chapter's
  binary exports OTLP to the host at the libvirt gateway (e.g.
  `http://192.168.124.1:4318`). Two things must line up: the otel-lgtm stack
  must **publish OTLP on the bridge IP**, not just `127.0.0.1` (see the
  compose file in Chapter 3), and the host firewall must **allow the guest
  subnet to reach those ports**. `virbr0` sits in firewalld's `libvirt` zone;
  for a trusted disposable guest the simplest fix is:
  ```bash
  sudo firewall-cmd --permanent --zone=libvirt --set-target=ACCEPT && sudo firewall-cmd --reload
  ```
  Symptom when this is wrong: the loader runs fine on the guest but no metrics
  appear in Grafana, and `curl http://192.168.124.1:4318/` from the guest is
  refused.

## Add the second guest (for two-host chapters)

Several networking chapters — `tcpconnlat`, `tcpstates`, the XDP load
balancer, TC traffic control — are only interesting with traffic
flowing between two real hosts. Provision a peer the same way:

```bash
cd scripts/lab && ./provision-vm.sh ebpf-peer
```

Because both guests attach to the same libvirt `default` network, they
are on the same subnet and can already reach each other. Confirm it —
get both IPs on the host, then ping the peer *from* the target:

```bash
T=$(./vm-ip.sh ebpf-target) && P=$(./vm-ip.sh ebpf-peer) && echo "target=$T peer=$P"
```

```bash
ssh fedora@"$T" "ping -c2 $P"
```

Two replies means the path between guests is open. The typical test
topology for the networking chapters is: the **peer** runs a client or
a server, the **target** runs your XDP/TC/socket eBPF program on the
interface facing the peer, and you watch the program's view of the
traffic in Grafana.

### What the networking part needs

This two-VM lab *is* the setup for the whole Networking part — there's no
separate networking install step. The chapters lean on it in three ways,
and each chapter's intro says which it uses:

| Need | Why | Used by |
|------|-----|---------|
| **Peer reachable** from the target | real host-to-host traffic to observe | tcpconnlat, tcpstates, http-l7, sockops, XDP/TC |
| **Interface name** on the target | XDP/TC/socket-filter programs attach to a NIC | http-l7, XDP, TC |
| **cgroup-v2** at `/sys/fs/cgroup` | `sock_ops` attaches to a cgroup | sockops |

All three come for free with a stock pair of guests: Fedora mounts
unified cgroup-v2 by default, `provision-vm.sh` installs the traffic
tools (`ncat`, `curl`, `socat`, `tcpdump`, `iproute`), and both guests
share the `default` NAT subnet. The only step beyond the single-VM setup
is provisioning the peer — so when a networking chapter says "bring up
the peer," it means exactly the one command above.

**Resource note:** two guests at the default 2 vCPU / 2 GB each want a
host with headroom — 8 GB RAM and 4 cores is comfortable. On a tight
laptop, shrink the peer (`VCPUS=1 RAM_MB=1536 ./provision-vm.sh
ebpf-peer`); the networking demos are not CPU-bound.

### Finding the interface to attach to

XDP and TC programs attach to a named interface. Inside the target,
the libvirt NIC is almost always `enp1s0` (virtio). Confirm before you
hardcode it anywhere:

```bash
ssh fedora@"$T" "ip -br link && ip -br addr"
```

Use the interface that carries the guest's lab IP. The example
`demo.sh` scripts detect this automatically rather than assuming a
name, but when you run a program by hand you'll pass it explicitly
(e.g. `--iface enp1s0`).

### Optional: an isolated lab network

The `default` NAT network is fine for everything in this tutorial. If
you'd rather keep lab traffic off the NAT (for example, to send
deliberately malformed packets in the XDP chapters without any chance
of them leaking out), define a second **isolated** libvirt network and
attach a second NIC to each guest. That's an optional refinement
introduced in the XDP part; the `default` network is the assumed
baseline until then.

## Tearing down and rebuilding

When you wedge a guest — and writing `sched_ext` schedulers, you will
— destroy and re-provision:

```bash
cd scripts/lab && ./destroy-vm.sh ebpf-target && ./provision-vm.sh ebpf-target
```

`destroy-vm.sh` removes the domain *and* its overlay disk and seed, so
the rebuild is genuinely clean. The cached base image stays, so the
rebuild is fast.

## What you should have now

- [x] `ebpf-target` running, reachable over SSH, BTF present, tooling
  installed
- [x] The deploy loop proven with a trivial binary
- [x] (Optional) `ebpf-peer` running and reachable from the target
- [x] The guest interface name noted for the networking chapters

[Next: Chapter 3 — The observability stack →]({{ "/docs/03-observability-stack/" | relative_url }})

---

*Verification status: every command and script in this chapter is
<span class="status status--unverified">unverified</span> until run on
a Fedora 44 laptop with libvirt. The Fedora Cloud image filename in
particular must be confirmed against the live mirror — see the callout
above. Record results in the
[reconciliation plan]({{ "/plans/reconciliation-plan/" | relative_url }}).*
