# Example 17 — sslsniff (TLS plaintext via libssl uprobes)

See the plaintext crossing TLS by probing OpenSSL — `SSL_write` before
encryption, `SSL_read` after decryption — even though the wire is
encrypted.

## What this shows

- Probing a **shared library** (`libssl.so.3`) rather than an
  executable.
- `SSL_write(ssl, buf, num)` — plaintext is in `buf` **at entry**, so a
  uprobe reads it directly.
- `SSL_read(ssl, buf, num)` — `buf` is only filled **by the time it
  returns**, so we stash `buf` at entry (uprobe) and read `ret` bytes
  from it at return (uretprobe). The entry/exit `HashMap` pattern again.
- Capturing a bounded chunk of binary data with `bpf_probe_read_user_buf`
  (capped at `DATA_CAP`).

## Run it

```bash
./demo.sh build     # build on host
./demo.sh           # build + deploy + attach to libssl on the VM
```

Confirm the symbols exist, then drive TLS on the VM:

```bash
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'nm -D /usr/lib64/libssl.so.3 | grep -E " SSL_(read|write)$"'
# any OpenSSL client works; e.g. a local TLS server + curl, or:
ssh fedora@"$(../../scripts/lab/vm-ip.sh ebpf-target)" 'openssl s_client -connect 127.0.0.1:443 </dev/null 2>/dev/null'
```

You'll see `WRITE`/`READ` rows with a printable preview of the
plaintext; `ebpf_events_total{program="sslsniff",dir=...}` in Grafana.

> Only processes using **this** libssl are captured. Apps statically
> linking OpenSSL, or using GnuTLS/NSS/BoringSSL/rustls, won't appear —
> point `LIBSSL=` at the right library, or probe that TLS library's
> equivalent functions.

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM: builds, loads, attaches all three programs to `libssl`, and runs
as described, capturing plaintext at `SSL_write` entry and `SSL_read`
return. The dynamic-length `bpf_probe_read_user_buf` read passes the
verifier with the constant `DATA_CAP` clamp. `SSL_read`/`SSL_write`
symbol names/offsets are OpenSSL-3-on-Fedora-44 specific, and attach
targets and struct offsets can be kernel-version-specific.

*Ethics: this is a debugging/observability tool for systems you operate.
Capturing other people's plaintext is exactly as sensitive as it
sounds — use it only on hosts and traffic you're authorized to inspect.*
