# Example 20 — javagc (JVM GC pauses by uprobing the G1 collector)

Time JVM garbage-collection pauses by attaching a **uprobe + uretprobe**
to the G1 collector's stop-the-world entry point in `libjvm.so`.

## What this shows

- **The textbook approach — and why it doesn't work here.** The "right"
  surface for JVM GC events is the HotSpot **USDT** probes
  (`hotspot:gc__begin` / `hotspot:gc__end`) — stable markers baked into
  `libjvm.so`. But those only exist in an OpenJDK built with
  `--enable-dtrace`, and **Fedora's (and most distros') OpenJDK is not**.
  On a stock JDK there are simply no gc USDT markers to attach to
  (`readelf -n libjvm.so | grep gc__` comes back empty).
- **The portable approach: uprobe the collector directly.** libjvm.so
  ships an unstripped `.symtab` (only the `.debug` is split out via
  `.gnu_debuglink`), so the collector's C++ symbols are resolvable. We
  uprobe `G1CollectedHeap::do_collection_pause_at_safepoint` on entry and
  put a **uretprobe** on its return — the difference is the **GC pause**.
  This catches *every* automatic G1 pause (young/mixed/full), not just
  explicit `System.gc()`.
- **aya resolves the symbol for us.** We pass the mangled name to
  `UProbe::attach(symbol, libjvm, ...)`; aya reads `.symtab` (and
  `.dynsym`) and computes the correct file offset — no hand-rolled
  vaddr→offset math. The name is JDK-version-specific and mangled, so the
  demo resolves it dynamically with `nm` rather than hard-coding it.
- Keyed by **tid**: the pause runs on the JVM's "VM Thread", so the
  uprobe entry and the matching uretprobe return share a kernel tid.

## Note on the lab policy

Our targets are normally containerized; here we run the JVM **directly
on the VM** to keep the focus on GC timing. To observe the containerized
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
./demo.sh           # compile+run Alloc.java on the VM, resolve the G1 symbol, time GC
```

You'll see per-pause lines (`VM Thread  0.97 ms`) and `jvm_gc_pause_ms`
in Grafana (heatmap / p99 — the classic "are GC pauses hurting latency?"
view).

## Cross-check — the same probe under bpftrace

```bash
[vm]$ LJ=$(find /usr/lib/jvm -name libjvm.so | head -1)
[vm]$ SYM=$(nm "$LJ" | awk '/do_collection_pause_at_safepoint/{print $3; exit}')
[vm]$ sudo bpftrace -e "uprobe:$LJ:$SYM { @b[tid]=nsecs }
       uretprobe:$LJ:$SYM /@b[tid]/ { @ms=hist((nsecs-@b[tid])/1000000); delete(@b[tid]) }"
```

## If you really want the USDT probes

Build an OpenJDK with `--enable-dtrace` (needs `systemtap-sdt-devel` at
build time) or use a vendor JDK that ships them; then the `hotspot:gc__*`
markers appear and you can attach a uprobe at each stapsdt `Location`
instead. Aya still has no first-class USDT helper, so the mechanics
(resolve offset → raw uprobe) are the same either way.

## ✅ Verification status

**Verified — Fedora 44, kernel 7.1.3, OpenJDK 26 (Red Hat build).**
Resolves `_ZN15G1CollectedHeap32do_collection_pause_at_safepointEm`,
attaches uprobe+uretprobe, and streams real GC pauses from the VM Thread
(~0.4–6 ms) while `Alloc` churns allocations — exported as
`jvm_gc_pause_ms`. aya 0.14 resolves the symbol from `.symtab`.
