# Example 20 — javagc (JVM GC pauses via HotSpot USDT probes)

Time JVM garbage-collection pauses by attaching to the HotSpot **USDT**
probes `hotspot:gc__begin` and `hotspot:gc__end`.

## What this shows

- **USDT** (User Statically-Defined Tracepoints): markers the JVM
  authors baked into `libjvm.so` at fixed instruction offsets, described
  in the ELF `.note.stapsdt` section. A USDT probe is just a **uprobe at
  that offset** — so we attach a plain `UProbe` with `fn_name = None`
  and the resolved offset.
- Why USDT beats uprobing the JVM directly: GC internals are C++
  (mangled, implementation-specific per collector) and partly JIT'd.
  The USDT probes are a **stable, documented** contract across JDK
  versions — the right surface for runtime events.
- Timing `gc__begin → gc__end` with the entry/exit pattern (two separate
  probe sites, keyed by pid — GC is stop-the-world per JVM) gives the
  **GC pause** duration.

## Note on the lab policy

Our targets are normally containerized; here we run the JVM **directly
on the VM** to keep the focus on USDT/GC. To observe the containerized
**Quarkus** target (Ch 16) instead, combine this with Chapter 16's
container-path resolution — find `libjvm.so` under the container's
overlay and use that path.

## Requires a JDK on the VM

```bash
[vm]$ sudo dnf install -y java-latest-openjdk-devel
```

## Run it

```bash
./demo.sh build     # build javagc on the host
./demo.sh           # compile+run Alloc.java on the VM, resolve USDT offsets, time GC
```

You'll see per-pause lines and `jvm_gc_pause_ms` in Grafana (heatmap /
p99 — the classic "are GC pauses hurting latency?" view).

## Cross-check — bpftrace definitely supports USDT

```bash
[vm]$ sudo bpftrace -e 'usdt:/path/to/libjvm.so:hotspot:gc__begin { @b[pid]=nsecs } usdt:/path/to/libjvm.so:hotspot:gc__end /@b[pid]/ { @ms=hist((nsecs-@b[pid])/1000000); delete(@b[pid]) }'
```

## ⚠ Verification status

**Unverified — most experimental chapter.** Risks:

1. **USDT offset resolution.** The demo parses `readelf -n` stapsdt
   notes for the probe `Location`, and assumes the uprobe file offset
   equals that vaddr (true for many shared objects, but verify — you may
   need the vaddr→file-offset conversion). If it can't resolve, the demo
   points you at the guaranteed-working `bpftrace` USDT command.
2. **USDT enabled in the JDK.** Most Linux OpenJDK builds ship the
   hotspot probes; `-XX:+ExtendedDTraceProbes` enables the fuller set.
3. `bpf_ktime_get_ns`, two-uprobe attach by offset, entry/exit HashMap
   in aya 0.13.x.

Aya has no first-class USDT helper yet, which is *why* we resolve the
offset ourselves and attach a raw uprobe — a useful thing to understand
regardless. Record results in `_plans/reconciliation-plan.md`.
