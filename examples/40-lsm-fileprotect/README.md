# 40 · Protecting a file with LSM

A BPF LSM program on `inode_permission` that makes one file **read-only even
for root** by denying `MAY_WRITE` on its inode — the defensive counterpart to
Chapter 39, and a first look at reading a kernel struct field from a BPF
program.

## What it does

- The loader `stat`s a file for its inode and writes it into `PROTECTED`,
  then loads `#[lsm(hook="inode_permission")] protect_file` against BTF.
- On each access check it filters to writes (`mask & MAY_WRITE`), reads
  `inode->i_ino` with `bpf_probe_read_kernel`, and returns `-EPERM` if the
  inode matches the protected one (bumping `DENIED`); otherwise allows.
  Read errors **fail open** so a bug can't wedge the box.
- Exports `ebpf_lsm_denied_total`.

> The `i_ino` offset is hard-coded and **kernel-version-specific** — verify
> with `pahole struct inode` or BTF. Part 9's CO-RE computes it at load time.

## Run it

```bash
./demo.sh                              # enable LSM + protect /tmp/ebpf-protected
./demo.sh build                        # just build on the host
PROTECT_FILE=/etc/myfile ./demo.sh     # protect a different file
```

Watch `/tmp/fileprotect.log` on the target: `READ-OK` and `WRITE-DENIED`.

## Verify on the target

```bash
stat -c '%i %n' /tmp/ebpf-protected
cat /tmp/ebpf-protected                 # reads work
echo x >> /tmp/ebpf-protected           # Operation not permitted
sudo sh -c 'echo x >> /tmp/ebpf-protected'   # denied even as root
```

## Verification status

**Unverified** — kernel ≥ 5.7 with `bpf` LSM active. Confirm: the
`inode_permission` `LsmContext` arg indexing (inode @0, mask @1, ret @2), the
`MAY_WRITE` value, the **`i_ino` offset in `struct inode`** for the running
kernel, and that `-1` refuses writes with `EPERM` even for root while reads
succeed.
