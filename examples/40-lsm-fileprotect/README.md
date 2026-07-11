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

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the lab
VM (kernel 7.1.3-200.fc44, which satisfies the kernel ≥ 5.7 + `bpf` LSM
requirement): builds, loads, attaches, and runs as described — writes to the
protected inode are refused with `EPERM` even for root while reads succeed. The
hard-coded **`i_ino` offset in `struct inode`** and the `inode_permission` arg
indexing are kernel-version-specific; confirm them against your kernel (CO-RE
in Part 9 computes the offset at load time).
