---
title: "Operating eBPF: lifecycle, pinning, and zero-downtime upgrades"
order: 59
part: Operating eBPF
description: "A program you load in a demo dies the instant the loader exits — fine for a chapter, fatal for a service. Operating eBPF means treating programs like deployable, upgradable services: they outlive the process that loaded them, survive a loader restart, swap to a new version without dropping a packet, and keep their state across the upgrade. The three pillars are pinning, link update, and pinned maps — and the orchestrators (L3AF, bpfman) that productize them."
duration: 45 minutes
---

Every `demo.sh` in this book has the same hidden assumption: the loader stays
running. Press Ctrl-C and the program detaches, the maps free, the data plane
goes dark — because a BPF object's lifetime is tied to the **file descriptors**
the loader holds, and when the process exits, those close. That's exactly right
for a demo and exactly wrong for a service. If your XDP filter is load-bearing
for production traffic, "restart the loader to deploy a new version" means a gap
where packets aren't filtered — or aren't forwarded at all. **Operating** eBPF,
the subject of this part now that one CO-RE artifact can target the whole fleet,
means making programs behave like deployable services: outliving their loader,
upgrading without a gap, and carrying their state across the upgrade. Three
kernel mechanisms make that possible, and two projects productize them.

The code is in `examples/59-bpf-lifecycle/`. `./demo.sh` pins a program so it
keeps running after the loader exits, then proves its state survived; the
`README.md` has the details.

{% include excalidraw.html
   file="bpf-lifecycle"
   alt="Operating eBPF: outlive the process, swap without a gap, keep the state. Three pillars. One, pinning for lifetime: pin program, link, and map to bpffs so they outlive the loader process and a new loader re-acquires them by path. Two, link update for zero-downtime: BPF_LINK_UPDATE swaps the program under a live link with no gap, no dropped packets or events. Three, pinned maps for state: reuse the same map across versions so counters and connection state survive the upgrade. By hand, a systemd unit pins and a deploy script updates the link; at fleet scale, L3AF (LF Networking) and bpfman (CNCF) productize exactly these three pillars."
   caption="Figure 59.1 — The three pillars of operating eBPF, and the orchestrators built on them" %}

## Pillar 1 — pinning decouples lifetime

Chapter 48 introduced pinning a map to **bpffs** so a second process could read
it. The same mechanism is the foundation of operating eBPF: pinning a program,
a **link**, or a map to `/sys/fs/bpf` takes a reference the kernel keeps even
after every userspace fd closes. Pin the right objects and the loader becomes
*disposable* — it sets things up, pins them, and can exit (or crash, or be
upgraded) while the data plane keeps running.

This unlocks two operational patterns:

- **The loader is no longer the lifetime.** A systemd unit can load and pin a
  program at boot; the daemon that manages it can be restarted, upgraded, or
  replaced without detaching the program. The pinned link in bpffs *is* the
  attachment now, independent of any process.
- **Privilege separation.** A privileged initializer pins the objects; later,
  less-privileged consumers `from_pin` them by path. In Kubernetes this is how a
  privileged setup container can hand eBPF maps to unprivileged workloads — the
  same idea behind the BPF token (Chapter 53), reached a different way.

In Aya the pieces are `EbpfLoader::map_pin_path` (pin a program's maps under a
directory as they load), `MapData::from_pin` (re-acquire one by path), and
turning an attachment into a pinnable `FdLink` and calling `.pin(path)`.

## Pillar 2 — links and atomic update give zero downtime

Early chapters attached programs the simple way and got back a `LinkId`. For
operating, the better handle is a **bpf_link** — a first-class kernel object
representing the attachment — because a link can be **updated in place**.
`BPF_LINK_UPDATE` atomically swaps the *program* a link points to: the new
program becomes live on the same hook **before** the old one is removed, so
there is no window where nothing is attached. On an XDP link that means **not a
single packet** sees an empty hook during the upgrade; on a tracepoint link it
means no events are missed.

That atomicity is the whole game for zero-downtime. The naive "detach old,
attach new" leaves a gap exactly the width of your deploy script; `link_update`
closes it to zero at the kernel level. Pin the link too, and the attachment
survives loader restarts *and* upgrades cleanly:

- **XDP/tc specifics**: XDP supports atomic replace flags, `tcx` links give
  ordered, atomically-managed attachment of multiple programs, and `libxdp`'s
  dispatcher composes several XDP programs on one interface — all variations on
  "swap without a gap."
- The program's *code* changes; the *attachment* (the link) and its *state*
  (the map, below) stay put.

## Pillar 3 — pinned maps carry state across the upgrade

Swapping the code is only half of a real upgrade. A connection tracker, a rate
limiter, an LRU of recent flows — these hold **state** you must not lose when
you deploy v2. The answer is to **reuse the same pinned map**: v2 attaches to
the map v1 created (by pin path) instead of creating a fresh one, so the
counters and tables carry straight over. `EbpfLoader::map_pin_path` plus
`MapData::from_pin` is the Aya expression of it: pin on first load, reuse on
every subsequent one.

This is the line between "restart the service" and "hot-swap the code." Without
map reuse, every upgrade is a cold start; with it, v2 picks up exactly where v1
left off — the same way our example will show a counter that keeps climbing
straight through the loader exiting and a new one taking over.

## Doing it by hand, and at fleet scale

On one machine you can assemble the three pillars yourself: a **systemd** unit
that loads and pins at boot, a deploy step that compiles v2 and calls
`link_update`, pinned maps for continuity. That's a perfectly good operating
model for a handful of hosts.

At fleet scale you reach for an orchestrator that productizes exactly this:

- **L3AF** (LF Networking; run in production by Walmart) provides **full
  lifecycle management** for eBPF programs at multiple hook points — loading,
  chaining several programs in a pipeline, configuration-driven deployment, and
  notably **graceful restart**: the control-plane daemon (`l3afd`) can be
  upgraded while the data-plane programs keep running, the new daemon taking
  ownership of the existing programs before the old one stops. Recent releases
  add container/Kubernetes support and CO-RE (Chapter 58) for portable packages.
- **bpfman** (CNCF) is a Kubernetes-native eBPF manager: a privileged daemon
  loads and pins programs and maps across nodes so unprivileged workloads can
  use them, centralizing the lifecycle the way an operator expects.

Both are the three pillars — pin, update, reuse — wrapped in an API and a
control plane. Knowing the primitives means you can read, debug, or replace
them rather than treating them as magic.

## Build, deploy, observe

```bash
cd examples/59-bpf-lifecycle && ./demo.sh
```

The demo runs a counter program, pins both its **link** and its **map**, then
exits the loader. The program keeps counting — `bpftool` shows the pinned map's
value still climbing with no loader attached — and a second loader run re-uses
the pinned map and continues from that value, not from zero. That sequence is
the three pillars made visible: the program outlived its process (pinned link),
and its state survived (pinned map). **In Grafana**, graph
`rate(ebpf_service_events_total[1m])` — the rate stays smooth across the loader
handoff, which is the zero-downtime property you're after.

## Cross-check

```bash
[vm]$ ls -l /sys/fs/bpf/ebpf-aya/                       # the pinned link + map
[vm]$ sudo bpftool link show                             # the attachment, with no owning process
[vm]$ sudo bpftool map dump pinned /sys/fs/bpf/ebpf-aya/EVENTS   # value climbing after exit
[vm]$ sudo bpftool prog show                             # the program still loaded
```

A pinned link in `bpftool link show` that no process owns, over a map whose
value keeps rising while the loader is gone, is the proof that lifetime is now
decoupled from the loader — the precondition for every upgrade pattern above.

## What you learned

- A BPF object's default lifetime is its loader's file descriptors; **operating**
  eBPF means decoupling that — **pinning** program/link/map to bpffs so they
  outlive (and survive upgrades of) the loader.
- **Links** make attachment a first-class object that **`BPF_LINK_UPDATE`** can
  swap **atomically** — the new program live before the old is gone, so an
  upgrade drops no packets or events — while **pinned maps reused across
  versions** carry state through the upgrade.
- By hand it's systemd + a deploy step + pinned maps; at fleet scale **L3AF**
  (lifecycle, chaining, graceful restart) and **bpfman** (Kubernetes-native
  management) productize the same three pillars.

Next, Chapter 60 turns to a very different frontier of operating eBPF —
**offloading work to hardware and accelerators**, where the program runs
somewhere other than the host CPU.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run: that `EbpfLoader::map_pin_path` pins maps and
`FdLink::pin` pins the link to bpffs; that the program keeps running and the map
keeps updating after the loader exits; that a second loader re-uses the pinned
map and continues the count; and treat `link_update`/atomic-swap ergonomics in
Aya as evolving — verify against the released API before relying on it for a
true in-place upgrade.*
