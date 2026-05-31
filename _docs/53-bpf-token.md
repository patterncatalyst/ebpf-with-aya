---
title: "The BPF token: delegating BPF into containers"
order: 53
part: Advanced kernel surface
description: "Loading BPF has always demanded capabilities in the init namespace — all or nothing for a container. The BPF token (kernel 6.9) breaks that open: a privileged runtime mounts a bpffs with four delegation axes, a trusted unprivileged container derives a token from it, and the kernel checks the token instead of init-namespace capabilities. Learn the capability problem it solves, the delegation model, and where it plugs into Aya."
duration: 40 minutes
---

There's a question this book has quietly assumed away in every chapter: the
loader runs as root. Each `demo.sh` deploys a binary to the lab VM and runs it
with `sudo`, because loading a program and creating maps needs BPF
capabilities — and those capabilities live in the **init namespace**. That
assumption falls apart the moment you want to run an eBPF tool *inside an
unprivileged container*: an observability agent in a pod, a sidecar that wants
to attach a socket filter. Until kernel 6.9 the only answers were bad ones. The
**BPF token** is the kernel's real answer, and it's the natural bridge from
this part into operating eBPF: a way to hand a container *exactly* the BPF
powers it needs and nothing more.

The code is in `examples/53-bpf-token/`. `./demo.sh` mounts a delegated bpffs
and shows the delegation policy the kernel will enforce; its `README.md` has
the details.

{% include excalidraw.html
   file="bpf-token"
   alt="Delegating BPF into a container without init-namespace privilege. A privileged runtime in the init user namespace mounts a bpffs with delegate_cmds, delegate_maps, delegate_progs, and delegate_attachs options. Inside a container user namespace with no init-namespace CAP_BPF, an unprivileged app opens that bpffs and calls BPF_TOKEN_CREATE to derive a token fd from it. The app then makes bpf() calls like PROG_LOAD and MAP_CREATE passing the token_fd, and the kernel checks them against the token, not init-namespace capabilities. The four axes — cmds, maps, progs, attachs — are the policy; the token grants only what was delegated: trust, not unconditional access."
   caption="Figure 53.1 — A delegated bpffs mints a token; the kernel checks the token, not init-ns caps" %}

## The capability problem

When eBPF arrived it required `CAP_SYS_ADMIN` — an enormous, do-anything
capability. Kernel 5.8 split out **`CAP_BPF`** for finer control, but loading
real programs still needs more: tracing programs (kprobe, tracepoint) want
`CAP_BPF + CAP_PERFMON`; networking programs (XDP, TC) want `CAP_BPF +
CAP_NET_ADMIN`. These are meaningful only in the **init user namespace**.

Now put that in a container. A container typically runs in its own **user
namespace**, where it may *look* like it has capabilities — but those caps are
namespaced, and BPF affects the *whole machine*: a program loaded in a pod sees
every packet and every task on the host, not just the pod's. There is, as the
kernel maintainers put it, no way to build a "mechanically verifiable
namespace-aware `CAP_BPF`" — a capability that means "BPF, but only for your
container," because BPF isn't scoped to a namespace. So the kernel can't simply
honor `CAP_BPF` inside a userns.

That left two options before 6.9, both bad:

- Give the container **real init-namespace privilege** (run it privileged, map
  its root to host root). Now a compromised container owns the host.
- Give it **nothing**, and run all your eBPF tooling as privileged host
  agents — losing the isolation containers exist to provide.

## What the token delegates

The BPF token replaces that binary choice with **delegation**. The model has
two parties and a deliberately narrow channel between them:

A **privileged runtime** in the init namespace (a container manager like
systemd, LXD/Incus, or a Kubernetes runtime) mounts a special, userns-bound
**bpffs** instance, configured with four **delegation axes** as mount options:

- **`delegate_cmds`** — which `bpf()` *commands* are allowed (e.g.
  `prog_load:map_create:btf_load:link_create`).
- **`delegate_maps`** — which *map types* may be created (e.g.
  `ringbuf:hash:array`).
- **`delegate_progs`** — which *program types* may be loaded (e.g.
  `socket_filter:xdp`).
- **`delegate_attachs`** — which *attach types* are allowed (e.g.
  `cgroup_inet_ingress`).

Each axis is a bitmask; a bit that isn't set is denied even with a token in
hand, and `any` opts into all of an axis. This *is* the security policy — the
host decides, per container, exactly which slice of the BPF subsystem is on the
table.

Inside the container, a **trusted unprivileged app** opens that bpffs and calls
**`bpf(BPF_TOKEN_CREATE)`** to derive a **token** — a file descriptor that
carries the delegated permission set. From then on, when the app issues
`bpf(BPF_PROG_LOAD)`, `bpf(BPF_MAP_CREATE)`, and friends, it passes the
**token fd** in the call. The kernel performs its permission check **against
the token**, not against the process's init-namespace capabilities. The app can
load programs and create maps — but only the types that were delegated.

Two properties make this safe rather than a privilege-escalation hole:

- **It's bounded to the user namespace.** Both the bpffs instance and the
  tokens derived from it are bound to their owning userns, so a token can't
  escape the container it was minted for.
- **Trust is explicit, not assumed.** As the maintainers stress, this is *not*
  unconditional unprivileged BPF — `CAP_BPF` is still checked in the container's
  *own* userns at token-creation time, and the host only delegates to
  containers it has chosen to trust. The kernel provides the mechanism; the
  surrounding infrastructure decides who is trusted and with what.

## How runtimes wire it up

The raw setup is genuinely fiddly — `fsopen`/`fsconfig` with
`FSCONFIG_CMD_CREATE`, `fsmount`, passing the mount fd into the container — so
in practice you never do it by hand. Container managers expose it as
configuration:

- **LXD / Incus**: `security.delegate_bpf=true` plus
  `security.delegate_bpf.prog_types`, `.map_types`, `.cmd_types`,
  `.attach_types` (Incus uses `security.bpffs.delegate_*`).
- **systemd** services can request a delegated bpffs; container runtimes are
  growing equivalent knobs.

The application inside simply finds a pre-delegated bpffs mounted at a known
path and derives its token from there. With **libbpf**, even that is
transparent — point it at the bpffs (`bpf_token_path` in the open options) and
it derives and threads the token through every `bpf()` call for you.

## Where Aya fits

Aya is on the same trajectory libbpf walked: the kernel mechanism is the
`token_fd` field on the relevant `bpf()` commands, and a loader needs to (1)
derive a token from a delegated bpffs and (2) thread its fd through program
load, map creation, and link creation. In libbpf that's a one-line
`bpf_token_path`; in Aya the equivalent loader option is **emerging** rather
than settled, so the example shows where it plugs into `EbpfLoader` and is
candid that you may be ahead of the released API. The important part for now is
conceptual: nothing about your *program* changes — the token is a property of
*how it's loaded*, supplied by the loader, so the same Aya program you wrote in
Part 1 can run in a delegated container once the loader threads a token.

## Build, deploy, observe

```bash
cd examples/53-bpf-token && ./demo.sh
```

The demo does the **privileged half** on the lab VM, which is the part you can
see plainly: it mounts a bpffs with a tight delegation policy
(`delegate_cmds=prog_load:map_create`, `delegate_progs=socket_filter`,
`delegate_maps=ringbuf`) and prints the mount so the four axes are visible.
That mount *is* the policy a container would derive a token from. There's no
Grafana panel here — this is a control-plane feature, not a data source — so
the "observation" is reading the delegation the kernel will enforce.

## Cross-check

```bash
[vm]$ uname -r                                   # need >= 6.9 for BPF token
[vm]$ mount | grep 'type bpf'                    # the delegate_* options on the mount
[vm]$ sudo bpftool feature probe | grep -i token # token support in this kernel
```

The `delegate_*` options visible on the mount are the cross-check: they're the
exact set of commands, maps, programs, and attach types a token derived from
this bpffs will permit — and nothing outside that set will load, token or not.

## What you learned

- Loading BPF needs capabilities (`CAP_BPF` + `CAP_PERFMON`/`CAP_NET_ADMIN`) in
  the **init namespace**, which a userns container can't safely hold — BPF
  isn't namespace-scoped, so there's no namespaced `CAP_BPF`.
- The **BPF token** (kernel 6.9) delegates a *subset* of BPF functionality: a
  privileged runtime mounts a bpffs with **`delegate_cmds`/`maps`/`progs`/
  `attachs`**, a trusted container derives a **token fd**, and the kernel
  checks the token instead of init-ns caps — bounded to the userns, trust
  explicit.
- The token is a property of **how a program is loaded**, not of the program;
  runtimes (LXD/Incus/systemd) wire it via config, libbpf threads it
  transparently, and Aya's loader support for it is emerging.

Next, Chapter 54 turns to BPF that schedules its own deferred work — **timers
and workqueues** running inside the kernel.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Confirm on a real Fedora 44 run (kernel ≥ 6.9): that bpffs accepts the
`delegate_*` mount options and they appear in `mount` output; that
`bpftool feature probe` reports token support; and treat the Aya `EbpfLoader`
token wiring as emerging — verify against the released API before relying on
it.*
