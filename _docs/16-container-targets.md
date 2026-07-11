---
title: "Bootstrap: containerized targets (Java & Python)"
order: 16
part: User-space & language probing
description: Stand up real application targets — Java 25 + Quarkus and Python 3.14 + FastAPI, both as multi-stage UBI containers — and learn to observe a process that lives inside a container by scoping eBPF to its cgroup, with crun's role on Fedora.
duration: 35 minutes
---

So far we've probed bash and a tiny Rust binary. Real targets are
*services*, and on Fedora they run in **containers**. This chapter
bootstraps two you'll reuse throughout the rest of the tutorial — a
**Java 25 + Quarkus** service and a **Python 3.14 + FastAPI** service,
both built as multi-stage UBI images — and teaches the thing that trips
people up the first time: how to observe a process that lives *inside* a
container with eBPF.

The code is in `examples/16-container-targets/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="container-observe"
   alt="Observing a containerized target: the VM kernel sees every container's processes; eBPF attaches by host PID or overlay path and scopes to one container by its cgroup id."
   caption="Figure 16.1 — observing a process inside a container" %}

## Where the containers run, and why

Until now, containers (the stack, the clients) ran on your **host**
laptop and eBPF ran on the **target VM**. The observed application
targets are different: to watch them with eBPF, they have to run where
your eBPF program runs — **on the target VM** — because eBPF attaches to
the kernel, and a process is only visible to the kernel it runs on.

So the topology for this chapter:

- **Target VM:** runs Podman, runs the Quarkus/FastAPI container, *and*
  runs your `contrace` Aya loader observing it.
- **Host:** runs the otel-lgtm stack and a load driver hitting the
  target's published port.

(That's why this iteration adds `podman`, `crun`, and `dwarves` to the
VM's cloud-init — re-provision `ebpf-target` so they're present.)

## The targets, as multi-stage UBI containers

Both targets follow the container policy: multi-stage, UBI base, slim
runtime. The **FastAPI** one resolves dependencies into a venv on
`ubi9/python-314`, then copies just the venv into
`ubi9/python-314-minimal`. The **Quarkus** one builds the fast-jar with
Maven + JDK 25, then runs it on the UBI OpenJDK 25 runtime. Each exposes
a `/work` endpoint that does a little CPU and opens a file — so there's
something for a probe to see under load.

> **Verify the OpenJDK image tag.** The Quarkus `Containerfile` targets
> `registry.access.redhat.com/ubi9/openjdk-25`. If that tag isn't
> published on your registry yet, use the fallback noted in the file's
> header (a JDK-25 builder of your choice + `ubi9/openjdk-21` or
> `ubi9-minimal` + a JRE at runtime). Pins like these are exactly what
> the reconciliation plan tracks.

## The key idea: the kernel sees every container

Here's the mental unlock. A container is not a VM — it's just processes
with their own **namespaces** (PID, mount, network, …) and a **cgroup**
for resource accounting, all sharing the host kernel. eBPF runs *in
that kernel*. So your existing tracers from Chapters 9–12 **already see
processes inside containers** — no container-specific API needed.

Two wrinkles come with that:

- **PID namespace.** The PID eBPF reports (`bpf_get_current_pid_tgid`)
  is the **host/VM** PID. Inside the container, `ps` shows a *different*
  PID (often the app is PID 1 in its namespace). When you correlate
  eBPF output with what you see inside the container, you must translate
  — the host PID is the source of truth for eBPF.
- **Everything at once.** Without a filter, you see every container plus
  the host's own processes. To watch *one* service you need to scope.

## Scoping to one container by cgroup id

The clean way to scope is the **cgroup**. Every container gets its own
cgroup, and `bpf_get_current_cgroup_id()` returns the current task's
cgroup id in-kernel. `contrace` uses it: user space writes the target
container's cgroup id into a one-slot config map, and the eBPF program
drops any event whose cgroup doesn't match.

```rust
let cgroup = bpf_get_current_cgroup_id();
let target = TARGET_CGROUP.get(0).copied().unwrap_or(0);
if target != 0 && cgroup != target {
    return Ok(());   // not our container — ignore
}
```

The eBPF side is otherwise a cgroup-filtered `opensnoop` (Chapter 9):
it traces `openat`, so each `/work` request that opens a file becomes a
scoped event. A target of `0` means "don't filter" — useful as a
fallback.

Finding a container's cgroup id from user space is the fiddly part. On
the VM:

{% raw %}
```bash
[vm]$ cgpath=$(podman inspect --format '{{.State.CgroupPath}}' fastapi-target)
[vm]$ stat -c %i "/sys/fs/cgroup${cgpath}"
```
{% endraw %}

The cgroup id is the **inode number** of the container's cgroup
directory. The exact `CgroupPath` layout varies between rootless and
rootful Podman and cgroup managers, which is why `contrace`'s demo
treats resolution as best-effort and falls back to unscoped if it can't
find it.

## crun: the runtime underneath

Fedora's default OCI runtime is **crun** (pinned at 1.27.1 here). It's
what Podman actually calls to create these containers. Three things
worth knowing for observation:

- crun puts each container in its own **cgroup v2** group — the very
  cgroups whose id `contrace` filters on.
- crun runs under **SELinux** confinement, and so do the containers it
  starts; observing them needs no special SELinux handling because
  you're watching from the kernel side, not from inside a peer
  container. If a future demo runs the *observer* itself in a container,
  that's when SELinux labels and extra capabilities (`CAP_BPF`,
  `CAP_PERFMON`) come into play — called out where it happens.
- crun supports eBPF-backed cgroup controllers itself; that's separate
  from our tracing, but it's why "eBPF" and "crun" show up in the same
  sentence in Fedora's docs.

## Build, deploy, observe

```bash
cd examples/16-container-targets && ./demo.sh            # FastAPI target
cd examples/16-container-targets && TARGET=quarkus ./demo.sh   # Quarkus target
```

The demo ships the target to the VM, builds and runs it under Podman
there, resolves its cgroup id, starts a host-side load driver hitting
`/work`, and runs `contrace` scoped to that container. You see only that
container's file opens, and
`ebpf_events_total{program="contrace",container="…"}` in Grafana — one
series per container, which is exactly the per-service view you want
when several are running.

**In Grafana** (`127.0.0.1:3000` → Explore), filter to the `ebpf-contrace` service and graph `sum by (program) (rate(ebpf_events_total[1m]))` — events captured from inside the container as a live rate, the same events your terminal lists, now plotted over time.

## Cross-check

```bash
[vm]$ sudo bpftrace -e 'tracepoint:syscalls:sys_enter_openat /cgroup == CGID/ { @[comm] = count(); }'
```

`bpftrace` has a `cgroup` builtin and a `cgroupid()` helper; scoping its
count to the same cgroup id should track what `contrace` reports.

## What you learned

- Observed targets run **on the VM** (Podman/crun) because eBPF attaches
  to that kernel; the kernel sees every container's processes.
- eBPF reports **host PIDs**, not in-container PIDs — translate when
  correlating.
- Scope observation to one container by **cgroup id**
  (`bpf_get_current_cgroup_id()` + a config map).
- crun is Fedora's runtime; its cgroup-v2 groups are what you filter on,
  and SELinux confinement is transparent when observing from the kernel.

These two targets return in later chapters — `sslsniff` against their
TLS, `funclatency` and `javagc` against the JVM, and the L7/HTTP
networking chapters. Next in this part: TLS data with **`sslsniff`** and
latency histograms with **`funclatency`**.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
