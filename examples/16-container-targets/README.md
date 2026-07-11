# Example 16 — container targets + cgroup-scoped observation (`contrace`)

Stand up two real application targets — **Java 25 + Quarkus 3.33** and
**Python 3.14 + FastAPI**, both multi-stage UBI containers — and observe
a *containerized* process with eBPF, scoped to one container by **cgroup
id**.

## The key idea: the kernel sees every container

eBPF runs in the kernel, which sees **all** processes regardless of
namespace. So our existing tracers (Ch 9–12) already see processes
inside containers — but with two wrinkles:

- **PID namespace:** the PID eBPF reports is the **host/VM** PID, not
  the PID you see with `ps` *inside* the container. User space may need
  to translate.
- **Scoping:** without a filter you see *every* container plus the host.
  To watch just one, filter by **cgroup id** with
  `bpf_get_current_cgroup_id()` — that's what `contrace` does.

The targets run **on the target VM** (under Podman/crun), because that's
the kernel our eBPF attaches to. The load driver runs on the host and
hits the target's published port.

## Pieces

```text
targets/fastapi/   # Python 3.14 + FastAPI, multi-stage UBI Containerfile
targets/quarkus/   # Java 25 + Quarkus 3.33, multi-stage UBI Containerfile
contrace-ebpf/     # openat tracepoint, FILTERED by cgroup id
contrace/          # user space: set target cgroup, attach, report
compose.yaml       # runs both targets (on the VM)
```

## Run it

```bash
./demo.sh                 # FastAPI target + scoped contrace
TARGET=quarkus ./demo.sh  # Quarkus target instead
```

The demo ships the target to the VM, builds + runs it under Podman
*there*, tries to resolve its cgroup id, starts a host-side load driver
hitting `/work`, and runs `contrace` scoped to that container. You'll
see only that container's file opens:

```
PID      CGROUP               COMM             FILE
13871    7842                 python           /etc/hostname
```

`ebpf_events_total{program="contrace",container="fastapi-target"}` in
Grafana.

## crun on Fedora

Fedora's default OCI runtime is **crun** (1.27.1). It launches these
containers, runs under SELinux confinement, and uses cgroup-v2 — the
same cgroups whose id we filter on. Nothing special is required to
observe a crun container; you're observing the kernel it shares with
everything else on the VM.

## ⚠ Verification status

**Unverified.** Highest-risk items:

1. **cgroup id resolution** — `podman inspect ... CgroupPath` + `stat -c
   %i` is best-effort and varies by rootless/rootful and cgroup manager.
   If it can't resolve, the demo runs **unscoped** (cgroup 0 = all),
   which still demonstrates the tool; scope manually with the commands
   the demo prints.
2. **UBI OpenJDK 25 / Quarkus 3.33 image + build** — verify the
   `ubi9/openjdk-25` tag exists; if not, use the fallback in the
   Quarkus `Containerfile` header.
3. `bpf_get_current_cgroup_id`, `Array::set`, and the openat offset
   (Ch 9) in aya 0.14.x.
4. Podman/crun present on the VM (added to cloud-init in this iteration —
   re-provision the target VM so it's installed).

Record results in `_plans/reconciliation-plan.md`.
