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
