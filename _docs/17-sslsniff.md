---
title: "sslsniff"
order: 17
part: User-space & language probing
description: See the plaintext crossing TLS by probing OpenSSL — SSL_write before encryption and SSL_read after decryption — learning to probe a shared library and to capture bounded binary data, with the ethics that come with it.
duration: 30 minutes
---

TLS encrypts traffic on the wire, so a packet sniffer sees ciphertext.
But the application handed OpenSSL **plaintext** to encrypt, and OpenSSL
handed the application **plaintext** after decrypting — and uprobes can
sit exactly at those two points. `sslsniff` attaches to `SSL_write` and
`SSL_read` in `libssl` and shows the cleartext, encryption
notwithstanding. It's the canonical demonstration of why uprobes on
crypto libraries are powerful — and sensitive.

The code is in `examples/17-sslsniff/`. `./demo.sh` there builds, deploys, and runs it; its `README.md` covers what it does and how to drive it.

{% include excalidraw.html
   file="tls-boundary"
   alt="TLS boundary capture: the wire carries ciphertext, but plaintext is present at SSL_write's buffer on entry and SSL_read's buffer on return, where sslsniff's uprobes read it."
   caption="Figure 17.1 — capturing plaintext at the TLS boundary" %}

> **Ethics first.** This captures plaintext — credentials, request
> bodies, anything an app sends over TLS. It's a legitimate debugging
> and security tool *on systems you operate and traffic you're
> authorized to inspect*. It is also exactly the capability malware
> wants. Use it only where you have the right to, and treat its output
> as sensitive.

## Probing a shared library

Chapters 14–15 probed executables. `libssl` is a **shared library**
(`/usr/lib64/libssl.so.3` on Fedora), used by curl, wget, language
runtimes, servers — anything doing OpenSSL TLS. A uprobe on a library
fires for **every** process that calls into it, which is what makes one
small program able to watch all OpenSSL traffic on the host. You attach
exactly as before, just pointing at the `.so`:

```rust
w.attach(Some("SSL_write"), 0, "/usr/lib64/libssl.so.3", None)?;
```

The catch is the same as Chapter 13: only processes using *this* library
are seen. Statically-linked OpenSSL, or GnuTLS / NSS / BoringSSL /
rustls, won't show up — you'd point at the relevant library and its
equivalent functions.

## Two functions, two timings

The plaintext is available at different moments for the two directions:

- **`SSL_write(ssl, buf, num)`** — the app passes `buf` *full of
  plaintext* to be encrypted. It's there **at entry**, so a plain
  uprobe reads it.
- **`SSL_read(ssl, buf, num)`** — the app passes an *empty* `buf` to be
  filled with decrypted plaintext. It's only populated **by the time
  the call returns**, and the return value is the byte count. So we
  stash `buf` at entry (uprobe) and read it at return (uretprobe) — the
  entry/exit `HashMap` correlation once more.

```rust
#[uprobe] pub fn ssl_write(ctx: ProbeContext) -> u32 {        // buf is plaintext now
    emit(DIR_WRITE, ctx.arg(1)?, ctx.arg(2)?); 0
}
#[uprobe] pub fn ssl_read_enter(ctx: ProbeContext) -> u32 {   // stash buf for later
    READ_BUF.insert(&pid_tgid(), &(ctx.arg::<*const u8>(1)? as u64), 0); 0
}
#[uretprobe] pub fn ssl_read_ret(ctx: RetProbeContext) -> u32 {// buf is filled now
    let buf = READ_BUF.get(&pid_tgid())?; emit(DIR_READ, buf, ctx.ret()?); 0
}
```

## FIPS mode doesn't change the boundary

A question that comes up in regulated environments: does **FIPS mode**
affect this? On RHEL and Fedora, `fips-mode-setup --enable` switches
OpenSSL to its FIPS-validated crypto provider and restricts the allowed
algorithms. That changes *which* ciphers negotiate and *who does the
math* — not *where the plaintext is*. `SSL_write` is still handed
plaintext at entry and `SSL_read` still returns plaintext, because those
functions sit **above** the provider boundary. So the same two uprobes
capture identically whether or not FIPS is enabled — you attach to
`libssl`'s read/write API, never to the provider (`fips.so`) underneath.

Two corollaries worth stating plainly:

- **It cuts both ways.** Probing the API boundary means a FIPS-validated
  cipher gives you no protection against *this* kind of observation —
  which is why the caution above matters more on a regulated system, not
  less. The validation covers the cryptography, not the plaintext your
  own process hands the library.
- **kTLS is what actually moves the boundary.** If an application enables
  *kernel* TLS offload (`setsockopt` `TLS_TX`/`TLS_RX`), bulk
  encrypt/decrypt moves into the kernel and the userspace
  `SSL_read`/`SSL_write` path can be bypassed for data transfer — then
  you'd trace the kTLS or socket layer instead. FIPS alone doesn't do
  that; kTLS does.

## Capturing bounded binary data

Unlike a filename, plaintext is arbitrary binary of arbitrary length.
We cap each capture at `DATA_CAP` (256) bytes and copy with
`bpf_probe_read_user_buf`, recording both the real length and how much
we captured:

```rust
let captured = core::cmp::min(len as usize, DATA_CAP);
bpf_probe_read_user_buf(buf_ptr, &mut (*ev).data[..captured]);
```

Reading a *dynamic* length into a buffer is where the verifier gets
fussy — it wants to prove the length can't exceed the destination. The
`min` with a constant cap is what makes it provable; if your kernel's
verifier still balks, clamp `captured` to a constant power of two and
mask. This is flagged as the chapter's main verification risk.

## Build, deploy, observe

```bash
cd examples/17-sslsniff && ./demo.sh
```

Confirm the symbols, then make some TLS happen on the VM — any OpenSSL
client will do:

```bash
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'nm -D /usr/lib64/libssl.so.3 | grep -E " SSL_(read|write)$"'
ssh fedora@"$(scripts/lab/vm-ip.sh ebpf-target)" 'openssl s_client -connect 127.0.0.1:443 </dev/null 2>/dev/null || true'
```

You'll see `WRITE` and `READ` rows with a printable preview of the
cleartext — an HTTP request line, a response header — captured at the
moment OpenSSL handled it. `ebpf_events_total{program="sslsniff",dir=…}`
splits reads from writes in Grafana.

**In Grafana** (`127.0.0.1:3000` → Explore), filter to the `ebpf-sslsniff` service and graph `sum by (program) (rate(ebpf_events_total[1m]))` — plaintext reads and writes around TLS as a live rate, the same events your terminal lists, now plotted over time.

## Cross-check

```bash
[vm]$ sudo sslsniff-bpfcc
```

The BCC tool does the same thing; run it alongside and the captured
plaintext should match.

## What you learned

- Probe a **shared library** by pointing the uprobe at the `.so`; it
  fires for every process using it.
- Plaintext is at `SSL_write`'s buffer **on entry** and `SSL_read`'s
  buffer **on return** — read the latter with the entry/exit pattern.
- Capture bounded binary data with a constant cap so the verifier can
  prove the bound.
- This power carries real responsibility — observe only what you're
  authorized to.

Next: timing functions with **`funclatency`**.

---

*Verification status: <span class="status status--unverified">unverified</span>.
Highest-risk: `bpf_probe_read_user_buf` with a dynamic captured length
(verifier bounds), three uprobes on one library, and OpenSSL 3 symbol
names on Fedora 44. The first build and run are the test.*
