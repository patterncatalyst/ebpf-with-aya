# 41 · Faking your way to root (lab-only offense)

**LAB-ONLY.** Forge the policy `sudo` reads: when a process named `sudo`
calls `read()` (its sudoers policy), overwrite the returned buffer with an
injected `NOPASSWD:ALL` line for a target user. `sudo` obeys a policy never
written to disk. Uses the kernel-tainting `bpf_probe_write_user`.

## What it does

- `enter_read` (tracepoint on `sys_enter_read`) stashes the buffer + count
  for `comm == "sudo"` reads.
- `exit_read` (tracepoint on `sys_exit_read`) overwrites the buffer with the
  payload line (ending in ` #` so the rest parses as a comment).
- The loader builds the line for a target user and exports
  `ebpf_sudo_tampered_total`.

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

- `cat /proc/sys/kernel/tainted` non-zero; `dmesg` warns about
  `bpf_probe_write_user`.
- `sudo bpftool prog show` lists tracepoints on `read`.
- Effective sudo rights disagree with on-disk `/etc/sudoers`.
- An LSM program can deny the `bpf` load that installs such a tool.

## Verification status

**Unverified** — confirm: `sys_enter`/`sys_exit_read` offsets (buf @24,
count @32, ret @16), `comm`-matching `sudo` catching the sudoers read, that
the buffer rewrite changes the parsed policy (payload length / trailing
comment may need tuning), and that `/proc/sys/kernel/tainted` flips.
