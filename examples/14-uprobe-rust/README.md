# Example 14 — uprobe on a Rust binary (`uprobe-rust`)

Attach a uprobe to a function in a Rust program you built, and read its
argument live as it runs.

## What this shows

- A **uprobe** (entry, vs. Ch 13's uretprobe) reading a function
  **argument** with `ctx.arg(0)`.
- The Rust angle: Rust mangles symbol names, so the target exports its
  function with `#[no_mangle]` + `extern "C"` — making the symbol
  literally `compute` and using the C calling convention so `arg(0)`
  reads the first parameter cleanly.
- Probing **your own binary**: the example ships a `target-app` you
  build, deploy, and attach to — the basis for tracing your own
  services in later chapters (`uprobe rust`, nginx, postgres).

## Pieces

```text
target-app/         # a normal binary with `#[no_mangle] extern "C" fn compute(x: u64)`
uprobe-rust-ebpf/   # uprobe on "compute" -> reads arg0 -> RingBuf
uprobe-rust/        # user space: attach to compute in the target binary, report
```

## Run it

```bash
./demo.sh build     # build snoop + target-app on the host
./demo.sh           # build, ship target-app to the VM, start it, attach the uprobe
```

The demo starts `target-app` on the VM (it calls `compute(i)` every
500 ms, i incrementing) and attaches the uprobe to it. You'll see:

```
PID      compute(arg0)
12345    compute(0)
12345    compute(1)
...
```

and `ebpf_events_total{program="uprobe-rust"}` climbing in Grafana.

## Cross-check (on the VM)

```bash
[vm]$ objdump -T /home/fedora/target-app | grep -w compute     # symbol is present + unmangled
[vm]$ sudo bpftrace -e 'uprobe:/home/fedora/target-app:compute { printf("arg0=%d\n", arg0); }'
```

## ⚠ Verification status

**Unverified.** Confirm: `ProbeContext::arg(0)` for a uprobe in aya
0.14.x; the `attach(Some("compute"), 0, path, None)` signature; that
`#[no_mangle] extern "C"` keeps the symbol attachable and `inline(never)`
preserves the call site under `--release` + LTO (if LTO inlines it away,
build the target-app without LTO or mark it `#[no_mangle]` only). A
mangled Rust function would need the mangled symbol and Rust's calling
convention — covered conceptually in the chapter. Record results in
`_plans/reconciliation-plan.md`.
