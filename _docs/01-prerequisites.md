---
title: Prerequisites
order: 1
part: Foundations
description: The hardware floor, the host OS, and the virtualization stack you need before building anything.
duration: 15 minutes
---

This chapter is the floor. By the end you will have confirmed that
your laptop can run KVM virtual machines, that rootless Podman works,
and that you have the handful of host packages the rest of the
tutorial assumes. Nothing here loads eBPF yet — that waits until you
have a throwaway VM to load it *into* (Chapter 2).

## Why a VM at all?

eBPF programs run **in the kernel**. A buggy program is usually caught
by the kernel's verifier and rejected safely, but not always: a bad
XDP program can black-hole your NIC, an LSM program can lock you out
of your own files, and a `sched_ext` scheduler can wedge your CPU
scheduling badly enough to require a hard reboot. On a laptop you use
for work, that is a bad afternoon. On a disposable Fedora 44 guest, it
is a `virsh destroy ebpf-target && ./provision.sh` and you are back in
ninety seconds.

So the model for the whole tutorial is:

<div class="lab-machines">
  <div class="lab-machine">
    <div class="lab-machine__role">Host · your laptop</div>
    <div class="lab-machine__name">Fedora 44 workstation</div>
    <p>Where you edit code (RustRover), build with Cargo, run Python clients in Podman, and run the Grafana/OTel stack. The kernel here is never the target.</p>
  </div>
  <div class="lab-machine">
    <div class="lab-machine__role">Guest · KVM</div>
    <div class="lab-machine__name">ebpf-target (Fedora 44)</div>
    <p>Where eBPF programs are loaded and attached. Disposable. Re-provisioned from a script whenever you wedge it.</p>
  </div>
  <div class="lab-machine">
    <div class="lab-machine__role">Guest · KVM (optional)</div>
    <div class="lab-machine__name">ebpf-peer (Fedora 44)</div>
    <p>A second guest, added in Chapter 2, for the chapters that need real traffic between two hosts (XDP, TC, tcpconnlat, the load balancer).</p>
  </div>
</div>

## Hardware floor

You are running one or two full Fedora VMs *plus* an all-in-one
Grafana stack *plus* Rust release builds. That is not heavy by modern
standards, but it is not nothing.

| Resource | Minimum (single target VM) | Comfortable (two VMs + stack) |
|----------|----------------------------|-------------------------------|
| CPU | 4 cores with virtualization | **8+ cores** |
| Memory | 16 GB | **32 GB** |
| Free disk | 40 GB | **80 GB** |
| Virtualization | VT-x/AMD-V enabled in firmware | same |

Each Fedora guest is comfortable at 2 vCPU / 4 GB / 20 GB disk. The
`grafana/otel-lgtm` container wants roughly 1–1.5 GB on its own. Rust
release builds of the Aya user-space crate are CPU-bound for a minute
or two the first time.

Check your laptop:

```bash
nproc && free -h && df -h ~ / && lscpu | grep -E 'Virtualization|Model name'
```

You want at least 4 CPUs, 16 GB total memory, 40 GB free where your
home directory lives, and a `Virtualization:` line reporting `VT-x`
(Intel) or `AMD-V`. If that line is missing, virtualization is
disabled in your firmware — reboot into BIOS/UEFI setup and enable
"Intel VT-x" / "SVM Mode", then re-check.

## Host operating system

This tutorial is written and tested against **Fedora 44** on the
laptop. Fedora derivatives (RHEL 10, Rocky 10, Alma 10) should work
for the host role with the same `dnf` commands; package names
occasionally differ, so verify with `dnf info <package>` first.

Confirm your Fedora version:

```bash
cat /etc/fedora-release && uname -r
```

You should see Fedora 44 and a 6.x kernel. The host kernel version
matters less than you might think — the eBPF programs run in the
*guest's* kernel, which you control independently.

## The virtualization stack: KVM, QEMU, libvirt

Fedora ships KVM (the in-kernel hypervisor), QEMU (the machine
emulator), and libvirt (the management layer) as a package group.
Install the group and the CLI tools the tutorial uses:

```bash
sudo dnf install -y @virtualization virt-install libvirt-client qemu-img guestfs-tools cloud-utils
```

`@virtualization` pulls in `libvirt`, `qemu-kvm`, and `virt-manager`.
`virt-install` and `qemu-img` are the command-line provisioning tools
Chapter 2 scripts against; `cloud-utils` provides `cloud-localds` for
building cloud-init seed images; `guestfs-tools` is handy for poking
at guest disks when something goes wrong.

Enable and start the libvirt daemon, then add yourself to the
`libvirt` group so you can manage VMs without `sudo`:

```bash
sudo systemctl enable --now libvirtd && sudo usermod -aG libvirt "$USER"
```

Group membership only takes effect on a new login session. Either log
out and back in, or start a fresh session for the current shell:

```bash
exec su - "$USER"
```

Verify KVM acceleration is actually available (not just installed):

```bash
virt-host-validate qemu | grep -Ei 'kvm|accel' && lsmod | grep kvm
```

You want `QEMU: Checking for hardware virtualization : PASS` and a
`kvm_intel` or `kvm_amd` module loaded. If `virt-host-validate`
reports a warning about cgroups or IOMMU, that is usually fine for
this tutorial; a hard `FAIL` on hardware virtualization is not — fix
the firmware setting from the hardware section above.

> **macOS note** — macOS is not a tested host platform here. You can
> run the *client* and *observability* pieces on macOS via Podman
> Desktop, but the KVM target VM has no macOS equivalent in this
> tutorial. If you only have a Mac, run the whole lab inside one Linux
> VM and treat that VM as the host.

## Rootless Podman (for clients and the stack)

The Grafana/OTel stack and the Python clients run in **rootless
Podman** on the laptop. Fedora ships Podman, but confirm it and prove
rootless works end to end:

```bash
sudo dnf install -y podman podman-compose && podman --version
```

```bash
podman run --rm registry.access.redhat.com/ubi9/ubi-minimal:latest echo OK
```

That should print `OK` with no `sudo`. It also proves you can pull UBI
images without any Red Hat subscription — which is the whole point of
standardizing on UBI for this tutorial.

> **SELinux and `:Z`** — Fedora runs SELinux in enforcing mode. Every
> bind mount in this tutorial carries the `:Z` suffix (e.g.
> `-v ./conf:/conf:Z`) so the container can read the host directory.
> It is correct on Fedora and a no-op — harmless — anywhere without
> SELinux.

## A quick word on kernel tooling

Chapters use `bpftool`, `bpftrace`, the `bcc` tools, and `perf` to
cross-check what your Aya programs see. As a project policy these come
**only from Fedora/Red Hat repositories**, never from upstream
binaries or random scripts:

```bash
sudo dnf install -y bpftool bpftrace bcc-tools perf
```

You install these *inside the target VM* in Chapter 2 (that's where
they're used), but installing them on the host too is convenient for
following along. Note that on the host you'd be inspecting the host
kernel; the meaningful runs happen in the guest.

## What you should have now

- [x] A Fedora 44 laptop with 4+ cores, 16+ GB RAM, VT-x/AMD-V on
- [x] `libvirtd` running, your user in the `libvirt` group,
  `virt-host-validate` passing on hardware virtualization
- [x] Rootless Podman pulling and running a UBI image
- [x] `virt-install`, `qemu-img`, `cloud-localds` available

If all four boxes are checked, you are ready to build the lab.

[Next: Chapter 2 — Lab setup →]({{ "/docs/02-lab-setup/" | relative_url }})

---

*Verification status: <span class="status status--verified">verified — Fedora 44 host</span>.
These prerequisites were installed and used to build and run the whole corpus
during the smoke campaign (Rust nightly + `rust-src`, `bpf-linker`, libvirt/KVM,
podman, the LGTM stack). If something here doesn't match what you see on your
own machine, that gap is exactly what the
[reconciliation plan]({{ "/plans/reconciliation-plan/" | relative_url }})
exists to record — open an issue with your output.*
