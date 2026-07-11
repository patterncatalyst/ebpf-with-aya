---
title: "Probing nginx: latency inside a real server"
order: 45
part: Application targets
description: "Turn the toolkit on production software: attach uprobes to a containerized nginx's request functions, key timing by the request object to measure per-request handling latency without touching nginx, and confront the real-world frictions — symbols and container namespaces."
duration: 45 minutes
---

The techniques so far were proven on small programs we wrote. The point of
eBPF is that they work just as well on software you *didn't* write and can't
change. This part aims the toolkit at production targets, starting with the
web server everyone runs: **nginx**. We'll measure how long nginx spends
handling each request — from inside the binary, with no log parsing, no
config change, no cooperation — by attaching **uprobes** to two of its
functions and keying the timing on the **request object** nginx passes
around. Along the way we hit the two frictions that make probing real apps
different from probing toy ones: **symbols** and **container namespaces**.

The code is in `examples/45-nginx-probe/`. `./demo.sh` there runs a
containerized nginx, drives load, and attaches the probe; its `README.md`
covers the details.

{% include excalidraw.html
   file="nginx-uprobe"
   alt="A client sends HTTP requests to an nginx worker running in a container. A uprobe on ngx_http_process_request records a start timestamp into STARTS keyed by the request pointer; nginx handles the request; a uprobe on ngx_http_finalize_request computes now minus start and adds it to a latency histogram exported to Grafana. The technique measures per-request latency inside a real server keyed by the request object with no app changes; you attach to the binary inside the container via the worker's pid and root, and the symbols must be present." 
   caption="Figure 45.1 — Per-request latency from inside nginx, keyed by the request object" %}

## The idea: key on the request object

nginx is event-driven — one worker process juggles thousands of connections,
so you can't use the PID (or even a thread) to pair "request started" with
"request finished." But nginx threads a single pointer through the whole
lifecycle: `ngx_http_request_t *r`, the request object. Every per-request
function takes it as its first argument. That pointer is a perfect key:
unique while the request is alive, and present at both ends.

So the plan is a uprobe on a function that runs near the **start** of request
processing (`ngx_http_process_request`) to stamp a start time keyed by `r`,
and a uprobe on a function that runs at the **end**
(`ngx_http_finalize_request`) to look up `r`, compute the elapsed time, and
record it. This is the funclatency idea from Chapter 18, but keyed by an
application object instead of a PID — which is what lets it work on a
concurrent server.

## How the code works

```rust
#[map] static STARTS: HashMap<u64, u64> = HashMap::with_max_entries(10240, 0); // r ptr -> start ns
#[map] static HIST:   HashMap<u32, u64> = HashMap::with_max_entries(64, 0);    // log2(us) -> count

#[uprobe]
pub fn req_start(ctx: ProbeContext) -> u32 {
    let r: u64 = ctx.arg(0).unwrap_or(0);          // ngx_http_request_t *r
    if r != 0 {
        let _ = STARTS.insert(&r, &unsafe { bpf_ktime_get_ns() }, 0);
    }
    0
}

#[uprobe]
pub fn req_done(ctx: ProbeContext) -> u32 {
    let r: u64 = ctx.arg(0).unwrap_or(0);
    if r == 0 { return 0; }
    if let Some(&start) = unsafe { STARTS.get(&r) } {
        let now = unsafe { bpf_ktime_get_ns() };
        let _ = STARTS.remove(&r);
        if now > start {
            let us = (now - start) / 1000;
            bump(&HIST, log2(us), 1);              // bucket by power of two
        }
    }
    0
}
```

Reading it:

- **`ctx.arg(0)`** in a uprobe reads the function's first argument from a
  register — here the `r` pointer. We use its numeric value as the map key;
  we never dereference it, so we don't need nginx's struct layout at all.
- **`req_start`** stamps `STARTS[r] = now`. **`req_done`** looks up `r`,
  computes the elapsed microseconds, drops it into a **log2 histogram**
  (Chapter 18's bucketing), and removes the entry so the map doesn't grow.
- Because the key is the request object, overlapping requests on the same
  worker never collide — exactly the property a PID key lacked.

The user side drains `HIST` into an ASCII latency histogram and exports
`ebpf_nginx_request_latency_us` buckets and `ebpf_nginx_requests_total`, so
Grafana shows nginx's request-latency distribution measured from inside the
process.

## The two frictions of real targets

### Symbols

A uprobe attaches to a *symbol* (or a raw offset). `ngx_http_process_request`
and `ngx_http_finalize_request` are internal nginx functions — present in the
binary's symbol table **only if it isn't stripped**. Distro nginx packages
are usually stripped, with symbols shipped separately as debuginfo. So step
zero on any real target is *checking*:

```bash
[vm]$ nm /usr/sbin/nginx | grep ngx_http_process_request   # in .symtab?
[vm]$ objdump -tT /usr/sbin/nginx | grep finalize_request
```

If those come up empty, you install the matching debuginfo (or build with
symbols) and attach by the resolved address. The example's container keeps
symbols so the probe can resolve them; the README shows how to verify.

### Container namespaces

nginx runs in a container, so its binary lives in the container's filesystem,
not at `/usr/sbin/nginx` on the host. A uprobe attaches to an **inode**, and
the inode you want is the one the worker is actually executing. The reliable
way to name it from the host is through the worker's process root:

```bash
[vm]$ pgrep -f 'nginx: worker'                 # the worker PID
[vm]$ ls -l /proc/<worker-pid>/root/usr/sbin/nginx   # the binary as the worker sees it
```

Attaching to `/proc/<pid>/root/usr/sbin/nginx` (optionally scoped to that
PID) points the uprobe at the exact inode the container is running. That
`/proc/<pid>/root` indirection is the key move for probing *any* containerized
binary — it's how you reach inside the namespace from the host where your
loader runs.

```rust
let target = format!("/proc/{worker_pid}/root/usr/sbin/nginx");
let start: &mut UProbe = ebpf.program_mut("req_start").unwrap().try_into()?;
start.load()?;
start.attach(Some("ngx_http_process_request"), 0, &target, Some(worker_pid))?;
// …same for req_done → ngx_http_finalize_request
```

## Build, deploy, observe

```bash
cd examples/45-nginx-probe && ./demo.sh
```

The demo builds a UBI nginx container that retains symbols, runs it on the
target, drives a load of HTTP requests, finds the worker PID, and attaches
both uprobes to the in-container binary. You'll see a live latency histogram
fill in and `ebpf_nginx_request_latency_us` populate in Grafana — nginx's own
request timing, measured without nginx's help.

## Cross-check

On the target (`[vm]$` — `ssh fedora@$(scripts/lab/vm-ip.sh ebpf-target)`):

```bash
[vm]$ nm /proc/$(pgrep -f 'nginx: worker' | head -1)/root/usr/sbin/nginx | grep process_request
[vm]$ curl -s -o /dev/null -w '%{time_total}\n' http://127.0.0.1:8080/   # client-side time
[vm]$ sudo bpftool perf show                     # the attached uprobes
```

The client-side `time_total` from `curl` should bracket the in-server latency
your probe reports (server time is a little less than total, which includes
the network) — agreement between the two is the probe working.

## What you learned

- **Probing software you didn't write**: uprobes on nginx's request functions
  measure per-request latency with no app changes or log parsing.
- **Keying by an application object** (the `ngx_http_request_t *r` pointer)
  instead of a PID is what makes timing work on a concurrent, event-driven
  server.
- The two real-world frictions: **symbols** (check `nm`/`objdump`, install
  debuginfo if stripped) and **container namespaces** (attach via
  `/proc/<pid>/root/...` to reach the in-container inode).

Next, Chapter 46 is a short capstone that ties the whole observability story
together — turning eBPF on Java and Python services to produce correlated
metrics, logs, and traces.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
