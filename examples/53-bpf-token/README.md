# 53 · The BPF token: delegating BPF into containers

Loading BPF needs `CAP_BPF` (+ `CAP_PERFMON`/`CAP_NET_ADMIN`) in the **init
namespace**, which an unprivileged container can't safely hold. The **BPF
token** (kernel 6.9) delegates a *subset* of BPF functionality to a trusted
container, scoped by four axes: `delegate_cmds`, `delegate_maps`,
`delegate_progs`, `delegate_attachs`.

## What this example shows

- `demo.sh` — the **privileged half**: mounts a bpffs with a tight delegation
  policy and prints it, so you can see exactly what a container's token would
  permit. (Control-plane feature — no Grafana panel.)
- `illustrative/loader_with_token.rs` — where a token plugs into an Aya loader.
  **Aya's loader-side token support is emerging**; libbpf threads it via
  `bpf_token_path`. The program is unchanged — the token is a property of *how*
  it's loaded.

## Run it

```bash
./demo.sh          # mount a delegated bpffs on $VM and show the policy (needs >= 6.9)
```

## The real path (container runtimes)

- **LXD/Incus**: `security.delegate_bpf=true` + `security.delegate_bpf.prog_types`,
  `.map_types`, `.cmd_types`, `.attach_types`.
- **systemd / container runtimes**: equivalent delegated-bpffs knobs.

## Cross-check

```bash
uname -r                                   # >= 6.9
mount | grep 'type bpf'                     # the delegate_* options
sudo bpftool feature probe | grep -i token
```

## Verification status

**Verified (privileged half) — Fedora 44, kernel 7.1.3.** On the lab VM,
bpffs accepts the `delegate_cmds`/`delegate_maps`/`delegate_progs` options and
reads them back in `mount`; an **invalid** axis (`delegate_cmds=not_a_real_cmd`)
is *rejected*, proving the kernel parses and enforces the policy rather than
ignoring it; and `BPF_TOKEN_CREATE` is present in the kernel ABI
(`/usr/include/linux/bpf.h`). Note `bpftool feature probe` (v7.6.0) does not
surface a "token" line — the delegated mount, not that probe, is the real
capability check.

The **Aya loader-side token API remains emerging**: confirmed against the
pinned **aya 0.14.0** — `EbpfLoader` exposes no `token_path`/token method (its
builder is `btf`/`override_global`/`map_pin_path`/`extension`/`load*`), so the
`illustrative/loader_with_token.rs` shape is still the intended-not-shipped
form. libbpf-based loaders thread this via `bpf_token_path` today; track the
Aya release notes for when the method lands.
