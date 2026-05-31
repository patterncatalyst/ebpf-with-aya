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

**Unverified** (kernel ≥ 6.9). Confirm bpffs accepts the `delegate_*` mount
options and they show in `mount`; that `bpftool feature probe` reports token
support; and treat the Aya `EbpfLoader` token wiring as emerging — verify
against the released API.
