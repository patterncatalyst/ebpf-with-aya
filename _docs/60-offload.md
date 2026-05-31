---
title: "Offload: running eBPF off the host CPU"
order: 60
part: Operating eBPF
description: "Every program so far has run on the host CPU in the kernel. Offload moves the work elsewhere — onto a SmartNIC's flow processor, a DPU's ARM Linux, an FPGA, or paired with GPU inference. Learn XDP's three modes and true hardware offload, why the practical 2026 answer is running ordinary eBPF on a DPU, where the FPGA and AI frontiers actually stand, and how Aya fits each."
duration: 40 minutes
---

Operating eBPF at scale eventually runs into a hard limit: the host CPU. An XDP
filter dropping a DDoS flood, a load balancer rewriting every packet — at
hundreds of gigabits these burn real CPU that your applications wanted. The
escape is **offload**: run the eBPF program *somewhere other than the host CPU*,
ideally close enough to the wire that unwanted packets never cost the host a
cycle. This is a genuinely exciting area and also one where the marketing runs
well ahead of what you can actually deploy, so this chapter is deliberately
careful about what's real, what's niche, and what's still research.

The code is in `examples/60-offload/`. `./demo.sh` attaches an XDP program,
asks for hardware-offload mode, and reports honestly what the lab NIC supports;
the `README.md` has the details.

{% include excalidraw.html
   file="offload"
   alt="Offload: run the program somewhere other than the host CPU. Packets from the wire arrive at a NIC or DPU where offloaded eBPF runs — off the host CPU — dropping or redirecting before the host; only some packets continue to the host CPU kernel, where native or generic XDP runs and sees only what the NIC passes. Where 'somewhere' can be: a SmartNIC flow processor that JITs eBPF into NIC machine code (Netronome/Corigine NFP); a DPU running ordinary eBPF on its ARM Linux, where Aya runs unchanged; FPGA synthesis via hXDP or eHDL (research); or GPU and Tensor cores doing ML inference on telemetry (NVIDIA Morpheus). True hardware offload is narrow (NFP); the practical 2026 answer is eBPF on a DPU's Linux; FPGA and GPU are the frontier."
   caption="Figure 60.1 — Offload targets, from the established SmartNIC path to the DPU answer to the research frontier" %}

## XDP's three modes

The clearest place to see offload is XDP, which can attach in **three modes**:

- **Generic (SKB) mode** — XDP emulated in the network stack after the SKB is
  built. No driver support needed, but slow; a correctness/testing fallback.
- **Native (driver) mode** — the program runs in the NIC driver, before the SKB
  exists, on the **host CPU**. This is "fast XDP," what Chapters 32–36 used.
- **Offload mode** — the program runs **on the NIC itself**, not the host CPU at
  all. Packets are processed by the card's own processor; the host may never see
  the ones the program drops.

The first two run on your CPU; only the third is offload in the literal sense.
Aya picks the mode with a flag at attach time (`XdpFlags::HW_MODE` for offload,
`DRV_MODE` for native, `SKB_MODE` for generic), and the example walks down that
list, which is how you discover what a given NIC supports.

## True hardware offload, and its narrow reality

Real XDP hardware offload means the loader hands the eBPF object to the driver
with a target device, and the driver's **JIT compiles eBPF into the NIC
processor's own machine code**. The flagship was **Netronome's Agilio (NFP)**
cards: their SDK JITs eBPF to NFP machine code and ships pre-built XDP/TC
offload functions for filtering, load balancing, and DDoS mitigation — with
reported throughput several times that of the same rules on the host CPU. The
verifier is stricter for offloaded programs, because the program can only use
the helpers and map types the NIC actually implements.

The honest caveat: **this hardware is narrow and waning.** NFP (now under
Corigine) is essentially the one general eBPF-offload NIC lineage, and even
prominent SmartNICs don't do it — NVIDIA's **BlueField-2**, for instance,
supports XDP in *driver* mode but its offload is "not tested/supported." So if
you write code expecting `HW_MODE` to engage, on the vast majority of NICs —
including the virtio NIC in this book's KVM lab — it simply won't, and you'll
fall back to native or generic. That's not a failure; it's the current state of
the hardware, and the example surfaces it rather than pretending otherwise.

## DPUs: the practical answer

The modern, deployable form of "offload" looks different. A **DPU** (data
processing unit) like NVIDIA's BlueField-3 isn't an ASIC you JIT into — it's a
**full computer on the NIC**: 16 ARM cores running their own Linux, sitting
between the network ports and the host. Its switching plane can forward packets
straight to the host like a normal NIC, *or* redirect them to the ARM cores
first. And on those cores you run **ordinary eBPF on ordinary Linux**.

That changes the engineering story completely, and in Aya's favour: there's no
special offload API to learn and no verifier subset to fight. You **cross-compile
your loader for the DPU's architecture** (`aarch64-unknown-linux-gnu`, or musl
per Chapter 4) and deploy it to the DPU's Linux exactly as you deploy to the lab
VM — the eBPF runs on the DPU, processing traffic before the host CPU is
involved, and **none of your Aya code changes.** For most teams in 2026, "run
eBPF on the DPU" is what offload actually means in production, and it's the one
form of it you can reach without exotic hardware support.

## The frontier: FPGA synthesis and GPU/AI

Two further directions are real but still research or specialized:

- **FPGA synthesis.** Projects like **hXDP** (an eBPF processor on an FPGA) and
  **eHDL** (which synthesizes a hardware pipeline directly from unmodified
  eBPF/XDP) run full eBPF *as hardware* on cards like the Alveo, reporting
  order-of-magnitude throughput gains. Powerful, but it's academic tooling, not
  something you `cargo build` today.
- **GPU and AI.** eBPF doesn't run *on* a GPU in production — but it pairs with
  one. NVIDIA's **Morpheus** uses the GPU and Tensor cores on a BlueField DPU to
  run ML inference over network telemetry that eBPF/XDP collects and pre-filters,
  for security at line rate. The division of labour is the point: **eBPF
  observes and filters near the wire; the accelerator does the inference.** Talk
  of "eBPF on GPUs" beyond this is research, and worth treating as such.

## Where Aya fits

Aya covers the part you can actually run today on both ends. For **mode
selection**, it sets `XdpFlags::HW_MODE`/`DRV_MODE`/`SKB_MODE` at attach, so you
can request offload and detect support. For **DPUs**, the answer is the whole
point of Part 9's CO-RE chapter: cross-compile and deploy — a DPU is just
another Linux target, and a CO-RE object plus a cross-built loader runs there
unchanged. True NFP offload would need that hardware and its toolchain; the
example is candid that the lab has neither.

## Build, deploy, observe

```bash
cd examples/60-offload && ./demo.sh
```

The demo attaches a packet-counting XDP program, asking for `HW_MODE` first,
then `DRV_MODE`, then `SKB_MODE`, and prints **which mode actually engaged** on
the lab's virtio NIC (expect native or generic — there's no offload NIC here).
The program is `XDP_PASS`, so it's safe on a live interface. **In Grafana**,
graph `rate(ebpf_offload_packets_total[1m])`; the metric is the same whatever
mode loaded — what changes on real hardware is *where those cycles are spent*,
which is the whole point of offload.

## Cross-check

```bash
[vm]$ ip -d link show dev <iface>          # shows xdp / xdpdrv / xdpoffload mode
[vm]$ sudo bpftool prog show                # 'offloaded_to' appears only on offload-capable HW
[vm]$ sudo bpftool net show                 # XDP attachments per interface
```

`ip -d link show` reporting `xdpdrv` (not `xdpoffload`) on the lab NIC is the
honest cross-check: the program loaded in native mode on the host CPU, because
there's no offload engine to take it — exactly what the chapter predicts.

## What you learned

- **Offload** runs eBPF off the host CPU; XDP's three modes are **generic**,
  **native** (both on the host CPU), and **offload** (on the NIC) — selected in
  Aya with `XdpFlags`.
- **True hardware offload** JITs eBPF into a NIC processor (Netronome/Corigine
  NFP) but the hardware is narrow and waning (BlueField does only driver-mode
  XDP); the **practical 2026 answer is a DPU** — a full ARM Linux on the NIC
  where **ordinary, unchanged Aya** runs, reached by cross-compiling (Chapter
  4/58).
- The **frontier** is FPGA synthesis (hXDP/eHDL, research) and **GPU/AI** where
  eBPF filters near the wire and an accelerator (e.g. NVIDIA Morpheus) does the
  inference — not eBPF running on the GPU itself.

Next, Chapter 61 turns to a quieter operating concern that offload also touches:
**power and efficiency** — measuring and reducing what eBPF (and the workloads
it watches) cost in energy.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that the XDP program attaches and the loader
correctly reports the engaged mode (expect `DRV`/`SKB` on virtio, not `HW`);
that `ip -d link show` reflects it; and treat all hardware-offload, DPU, FPGA,
and GPU claims as descriptions of external hardware/projects this lab can't
exercise — only the mode-selection path is runnable here.*
