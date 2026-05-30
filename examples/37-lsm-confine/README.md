# 37 · BPF LSM: from observing to deciding

A BPF LSM program that **denies outbound connections** for processes in a
confined cgroup — the first program in this book whose return value is an
access-control decision (0 = allow, -EPERM = deny), scoped so it confines
one container without touching the host.

## What it does

- Attaches `#[lsm(hook = "socket_connect")] confine_connect` (loaded against
  kernel BTF).
- For each `connect()`, if the caller's `bpf_get_current_cgroup_id()` is in
  the `CONFINED` map, returns `-EPERM` (and bumps `DENIED`); otherwise `0`.
- The loader stats a cgroup-v2 path for its id and confines it, then exports
  `ebpf_lsm_denied_total`.

## Run it

```bash
./demo.sh                         # enable LSM + confine /sys/fs/cgroup/confined + drive curls
./demo.sh build                   # just build on the host
CONFINE_CGROUP=/sys/fs/cgroup/foo ./demo.sh   # confine a different cgroup
```

Needs the target on kernel ≥ 5.7 with the **bpf LSM active**
(`scripts/lab/enable-bpf-lsm.sh ebpf-target`, which the demo runs). Watch
`/tmp/confine.log` on the target: confined curls print `CONFINED-BLOCKED`,
normal curls print `HOST-OK`.

## Verify on the target

```bash
cat /sys/kernel/security/lsm                  # must include "bpf"
sudo bpftool prog show | grep lsm             # the lsm program is loaded
sudo bash -c 'echo $$ > /sys/fs/cgroup/confined/cgroup.procs; curl -m2 http://example.com'
# -> curl: (7) Couldn't connect  (EPERM from connect)
```

## Verification status

**Unverified** — kernel ≥ 5.7 with `bpf` in the LSM list. Confirm: the Aya
LSM API (`#[lsm(hook)]`, `Lsm::load(hook, &btf)`, `attach()`), the
`LsmContext` arg indexing (trailing `ret` at index 3 for `socket_connect`),
that returning `-1` fails `connect` with `EPERM`, and that a cgroup-v2
directory's inode equals `bpf_get_current_cgroup_id()` (may need
`name_to_handle_at`).
