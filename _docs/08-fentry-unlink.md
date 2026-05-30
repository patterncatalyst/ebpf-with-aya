---
title: "fentry + unlink"
order: 8
part: Tracing the kernel
description: Revisit the unlink target with fentry/fexit — BTF-trampoline probes that are lower overhead than kprobes, give typed argument access, and let fexit read the function's return value, so we can report whether each unlink actually succeeded.
duration: 30 minutes
---

Chapter 7's kprobe told you an unlink was *attempted* and dredged the
filename out of a raw pointer. This chapter attaches to the same
function — `do_unlinkat` — with **fentry** and **fexit**, and gets two
things the kprobe couldn't: lower overhead with typed arguments, and
the function's **return value**, so we can report whether each delete
actually *succeeded*. The program, `fentrysnoop`, is the kprobe chapter
leveled up.

The code is in `examples/08-fentry-unlink/`.

{% include excalidraw.html
   file="entry-exit"
   alt="Entry/exit correlation: a probe at function entry stashes an argument or timestamp in a HashMap keyed by pid_tgid; a probe at return reads it back to pair the two events."
   caption="Figure 8.1 — the entry/exit correlation pattern (reused throughout the book)" %}

## What fentry/fexit are

A kprobe works by patching a breakpoint (`int3`) into the function's
machine code; when the CPU hits it, it traps into your program. It's
universal but comparatively heavy, and the arguments arrive as raw
`pt_regs` you interpret by hand.

**fentry** and **fexit** instead use a BTF-generated **trampoline**:
the kernel knows the target function's type signature (from BTF), so it
can hand your program **typed arguments** directly, with far less
overhead than a trap. **fexit** runs at function *return* and can read
the **return value** — which a kprobe entry simply cannot see (you'd
attach a second `kretprobe` for that). The cost: fentry/fexit require
the kernel to have BTF (the `/sys/kernel/btf/vmlinux` you confirmed in
Chapter 2) and a reasonably recent kernel — both true on Fedora 44.

| | kprobe (Ch 7) | fentry/fexit (Ch 8) |
|---|---|---|
| Mechanism | `int3` breakpoint | BTF trampoline (lighter) |
| Argument access | raw `pt_regs` | typed, via BTF |
| Return value | needs a separate kretprobe | fexit reads it directly |
| Requires kernel BTF | no | **yes** |

## The plan: bridge entry and exit

A function's arguments are available at *entry*; its return value at
*exit*. To report both for the same call, we record the entry data,
then look it up at exit and attach the return value. The bridge is a
`HashMap` keyed by `pid_tgid` — the same value `bpf_get_current_pid_tgid()`
returns in both probes for the same call:

```text
do_unlinkat ENTRY (fentry) ──> capture pid/uid/comm/filename
                               store in INFLIGHT[pid_tgid]
do_unlinkat RETURN (fexit) ──> look up INFLIGHT[pid_tgid]
                               attach return value
                               emit completed event, clear entry
```

This correlate-two-probes pattern recurs constantly (it's exactly how
latency tools like `runqlat` in Chapter 21 pair a start and end
timestamp), so it's worth getting comfortable with here on something
simple.

## The kernel side

`fentrysnoop-ebpf/src/main.rs` defines two programs and two maps. The
entry probe gathers context and stashes it:

```rust
#[fentry(function = "do_unlinkat")]
pub fn do_unlinkat_enter(ctx: FEntryContext) -> u32 {
    // pid/uid/comm + filename (arg 1), then INFLIGHT.insert(&pid_tgid, &ev, 0)
    0
}
```

The exit probe reads the return value and emits the completed record:

```rust
#[fexit(function = "do_unlinkat")]
pub fn do_unlinkat_exit(ctx: FExitContext) -> u32 {
    let id = bpf_get_current_pid_tgid();
    if let Some(stored) = unsafe { INFLIGHT.get(&id) } {
        let mut ev = *stored;
        ev.ret = ctx.arg::<i64>(2).unwrap_or(0) as i32;   // return value follows the 2 args
        // reserve a RingBuf slot, write ev, submit
        INFLIGHT.remove(&id).ok();
    }
    0
}
```

The one genuinely new idea is `ctx.arg::<i64>(2)`. In an fexit program,
the return value is exposed *after* the function's arguments —
`do_unlinkat` takes two, so index `2` is the return. A `0` means the
unlink succeeded; a negative value is a `-errno` (e.g. `-2` is
`ENOENT`, "no such file"). This is the bit most likely to need a tweak
on your kernel/aya version, so it's flagged in the reconciliation plan.

The filename read is the same technique as Chapter 7 — and carries the
same `struct filename` layout caveat. fentry gives you *typed
arguments*, which makes the pointer's type trustworthy, but following a
nested pointer to a string still goes through `bpf_probe_read_kernel`.
The fully robust version uses BTF-generated kernel types (via
`aya-tool generate`) so field offsets relocate automatically; that's
the CO-RE machinery built out in Chapter 56. Here we keep the read
explicit so the mechanic stays visible, and degrade to an empty
filename if it fails.

## The user side

Loading fentry/fexit differs from a kprobe in one way: they need the
kernel's BTF to resolve the target function's type. `fentrysnoop/src/main.rs`:

```rust
let btf = Btf::from_sys_fs()?;                 // reads /sys/kernel/btf/vmlinux

let enter: &mut FEntry = ebpf.program_mut("do_unlinkat_enter").unwrap().try_into()?;
enter.load("do_unlinkat", &btf)?;
enter.attach()?;

let exit: &mut FExit = ebpf.program_mut("do_unlinkat_exit").unwrap().try_into()?;
exit.load("do_unlinkat", &btf)?;
exit.attach()?;
```

`Btf::from_sys_fs()` is why Chapter 2 made you confirm
`/sys/kernel/btf/vmlinux` exists — without it, fentry/fexit can't
load. Draining the ring buffer is identical to Chapter 7; the only
addition is a `result="ok"|"fail"` label on the exported counter so you
can chart success versus failure:

```rust
let result = if ev.ret == 0 { "ok" } else { "fail" };
counter.add(1, &[KeyValue::new("program", "fentrysnoop"), KeyValue::new("result", result)]);
```

## Build, deploy, observe

```bash
cd examples/08-fentry-unlink && ./demo.sh
```

The demo deliberately generates both successful deletes *and* a failing
one (`rm` of a nonexistent path) so you see a non-zero `RET`:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'for i in $(seq 1 10); do t=$(mktemp); rm -f "$t"; done; rm -f /nonexistent-$RANDOM 2>/dev/null || true'
```

The table shows a `RET` column; the successful `mktemp`/`rm` pairs
report `0`, the bogus removal reports a negative errno. In Grafana, the
`result` label splits `ebpf_events_total` into ok versus fail — a
distinction the Chapter 7 kprobe couldn't make.

## Cross-check against the kernel

```bash
[vm]$ sudo bpftrace -e 'fexit:do_unlinkat { @[retval == 0] = count(); }'
```

`bpftrace`'s `fexit` probe exposes `retval` directly; this one-liner
counts successes versus failures independently. If its split matches
the `ok`/`fail` split in your Grafana panel, both your fexit
return-value read and your entry/exit correlation are correct.

## What you learned

- fentry/fexit are BTF-trampoline probes: lighter than kprobes, typed
  arguments, and fexit can read return values.
- A `HashMap` keyed by `pid_tgid` bridges an entry probe to an exit
  probe — the foundation of every latency-measuring tool to come.
- The same program now reports *outcomes*, not just attempts.

Next we move from unlink to the file-open path with **`opensnoop`**,
and start building per-event tooling in earnest.

---

*Verification status: <span class="status status--unverified">unverified</span>.
The fexit return-value index, the `FEntry`/`FExit` load/attach API, and
the filename read are unrun at authoring — see the README's
verification notes. The first `cargo build` and `./demo.sh` on Fedora
44 are the test.*
