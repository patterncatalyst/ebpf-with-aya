# 45 · Probing nginx: latency inside a real server

Attach uprobes to a containerized **nginx** and measure per-request handling
latency from inside the binary — no log parsing, no config change — keyed by
the request object so it works on a concurrent, event-driven server.

## What it does

- `req_start` (uprobe on `ngx_http_process_request`) stamps a start time in
  `STARTS` keyed by the request pointer (`ctx.arg(0)`).
- `req_done` (uprobe on `ngx_http_finalize_request`) computes the elapsed
  time and buckets it into a log2(us) histogram `HIST`.
- The loader prints a live histogram and exports
  `ebpf_nginx_request_latency_us{le}` + `ebpf_nginx_requests_total`.

## Run it

```bash
./demo.sh          # build symbol-keeping UBI nginx on $VM + load + attach uprobes
./demo.sh build    # just build the probe on the host
```

The `Containerfile` builds nginx from source with `--with-debug -g` so its
symbols are present for uprobe resolution (the first run compiles nginx and
is slow). The demo finds the worker and attaches to
`/proc/<worker-pid>/root/usr/sbin/nginx`.

## The two frictions of real targets

- **Symbols:** `nm <binary> | grep ngx_http_process_request` must find it.
  Distro nginx is usually stripped — build with symbols or install debuginfo.
- **Container namespaces:** attach via `/proc/<pid>/root/...` to reach the
  exact inode the container runs.

## Verify on the target

```bash
nm /proc/$(pgrep -f 'nginx: worker'|head -1)/root/usr/sbin/nginx | grep process_request
curl -s -o /dev/null -w '%{time_total}\n' http://127.0.0.1:8080/   # brackets server time
sudo bpftool perf show
```

## Verification status

**Unverified** — confirm the nginx build exposes `ngx_http_process_request` /
`ngx_http_finalize_request` in `.symtab`, the Aya `UProbe::attach(fn, offset,
target, pid)` signature, that `/proc/<pid>/root/...` resolves the
in-container binary, that `ctx.arg(0)` is the `r` pointer on this ABI, and
that the histogram tracks client-observed latency.
