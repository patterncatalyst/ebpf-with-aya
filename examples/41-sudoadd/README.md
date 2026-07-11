# 41 · Faking your way to root (lab-only offense)

**LAB-ONLY.** Forge the policy `sudo` reads: when a process named `sudo`
calls `read()` (its sudoers policy), overwrite the returned buffer with an
injected `NOPASSWD:ALL` line for a target user. `sudo` obeys a policy never
written to disk. Uses the kernel-tainting `bpf_probe_write_user`.

## What it does

- The loader captures the first 16 bytes of `/etc/sudoers` as a **signature**
  and hands it to the program, then builds the injected line for the target
  user and exports `ebpf_sudo_tampered_total`.
- `enter_read` (tracepoint on `sys_enter_read`) stashes the buffer + count for
  `comm == "sudo"` reads.
- `exit_read` (tracepoint on `sys_exit_read`) overwrites the buffer **only when
  it begins with the sudoers signature** — i.e. a read of the file header
  (offset 0) — with the payload line (ending in ` #` so the rest of the line
  parses as a comment).

### Why the signature, not "every sudo read"

Two things a naive "overwrite every `read()` by `sudo`" gets wrong, both found
on a live Fedora 44 box (kernel 7.1.3, sudo 1.9.17):

- **It bricks sudo.** The dynamic loader `read()`s shared-library ELF headers
  under `comm == "sudo"` at startup; clobbering one (`libaudit.so.1: invalid
  ELF header`) kills sudo before it ever parses a policy — and since only
  `sudo` reads are corrupted, you can't even `sudo pkill` to recover. Matching
  the sudoers header leaves library reads untouched.
- **It misses the parse.** sudo reads `/etc/sudoers`, then `lseek()`s back to 0
  and **re-reads** it for the actual parse. Tampering only the first read (or
  unmarking the fd after one hit) leaves the parse read clean. Because the file
  on disk is never modified, *every* offset-0 read still matches the signature,
  so the parse read is tampered too.

## Run it

```bash
./demo.sh                          # create 'victim', forge its sudo rights, exercise sudo
./demo.sh build                    # just build on the host
TARGET_USER=alice ./demo.sh        # target a different user
```

### See the escalation (on the target, while attached)

```bash
sudo -u victim sudo -n id          # -> uid=0(root) ... while attached
cat /etc/sudoers | grep victim     # -> nothing: the file on disk is clean
```

## Detecting it

- `sudo bpftool prog show` lists tracepoints on `read` — the most reliable
  tell; there's no benign reason for one on a server.
- Effective sudo rights disagree with on-disk `/etc/sudoers`.
- An LSM program can deny the `bpf` load that installs such a tool.
- **Note:** on kernel 7.1.3, `bpf_probe_write_user` does **not** taint the
  kernel or emit a `dmesg` warning (verified: `/proc/sys/kernel/tainted` stays
  `0`, no journal notice) — don't rely on a passive taint flag for detection.

## Verification status

**Verified — Fedora 44, kernel 7.1.3, sudo 1.9.17p2.** Built on the host,
run on the lab VM: with the tool detached, `victim` cannot sudo; while
attached, `sudo -u victim sudo -n id` returns `uid=0(root)` on the first
attempt while sudo itself stays healthy; on detach, `victim` is denied again
and `/etc/sudoers` on disk is unchanged. The verifier rejected the original
program on this kernel (`bpf_probe_write_user`'s size is `ARG_CONST_SIZE`, so a
possibly-zero length is refused — "R3 invalid zero-sized read"); the size is
now clamped to `1..=64`.
