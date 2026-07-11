# Example 08 — fentry + unlink (`fentrysnoop`)

The same `vfs_unlink` target as Chapter 7, but with **fentry + fexit**
instead of a kprobe — so we can report whether each unlink *succeeded*,
not just that it was attempted.

## What this shows

- `#[fentry]` and `#[fexit]` on `vfs_unlink` — BTF-trampoline attach
  points, lower overhead than kprobes, with typed argument access.
- **fexit reads the return value** (0 = success, negative errno =
  failure). A single kprobe entry can't see the return; you'd need a
  separate kretprobe. fentry/fexit pairs are cleaner.
- Bridging entry → exit with a **`HashMap`** keyed by `pid_tgid`
  (`INFLIGHT`), the classic correlate-two-probes pattern.
- Exporting `ebpf_events_total{program="fentrysnoop",result="ok|fail"}`
  so you can chart success vs. failure in Grafana.

## fentry/fexit vs kprobe (Chapter 7)

| | kprobe (Ch 7) | fentry/fexit (Ch 8) |
|---|---|---|
| Mechanism | int3 breakpoint | BTF trampoline (lower overhead) |
| Arg access | `pt_regs`, manual | typed via BTF |
| Return value | needs separate kretprobe | fexit reads it directly |
| Requires BTF | no | **yes** (you confirmed it in Ch 2) |

## Run it

```bash
./demo.sh build      # just build on the host
./demo.sh            # build + deploy to the VM + run (Ctrl-C to stop)
```

Generate successful and failing unlinks on the target:

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'for i in $(seq 1 10); do t=$(mktemp); rm -f "$t"; done; rm -f /nonexistent-$RANDOM 2>/dev/null || true'
```

You'll see a `PID UID RET COMM FILE` table; the deliberate
`/nonexistent-*` removal shows a non-zero `RET`. In Grafana, the
`result` label lets you split the event counter into ok vs. fail.

## Cross-check (on the VM)

```bash
[vm]$ sudo bpftrace -e 'fexit:vfs_unlink { @[retval == 0] = count(); }'
```

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches both
the fentry and fexit programs cleanly, and runs as described. The fexit
return-value read (`ctx.arg::<i64>(4)`, the return following `vfs_unlink`'s
four args), the `FEntry`/`FExit` load+attach path, and the filename read
all behaved as written. fentry/fexit require kernel BTF and a recent
kernel — both satisfied here — and attach targets and struct offsets can
be kernel-version-specific.
