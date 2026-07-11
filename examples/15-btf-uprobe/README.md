# Example 15 — BTF-assisted uprobe (`btf-uprobe`)

Read a **struct** argument out of a running program, and understand the
role BTF plays in getting the layout right when you don't control the
target.

## What this shows (new vs. Ch 13–14)

- Reading a **pointer-to-struct** argument, not a scalar: take `arg(0)`
  as a `*const Order`, then `bpf_probe_read_user::<Order>(ptr)` to copy
  the whole struct out of the target's memory.
- The **layout contract** made central: the probe and the app must
  agree on `Order`'s `#[repr(C)]` layout. Here they share the definition
  (`btf-uprobe-common`) so it's correct by construction.
- **BTF's role**: when you *can't* share the definition (probing a
  binary you didn't write), BTF is how you recover the layout — dump the
  target's BTF, generate the mirror from it, and (with user-space CO-RE)
  relocate offsets so your probe survives the struct changing.

## Pieces

```text
btf-uprobe-common/   # the shared #[repr(C)] Order + OrderEvent
target-app/          # calls process_order(&Order) in a loop; built with debug info
btf-uprobe-ebpf/     # uprobe on process_order -> reads *const Order -> RingBuf
btf-uprobe/          # user space: attach, decode Order fields, report
```

## Run it

```bash
./demo.sh build     # build snoop + target-app on the host
./demo.sh           # build, ship target-app to the VM, start it, attach the uprobe
```

Output:

```
PID      ID       AMOUNT       STATUS
12345    1000     $0.00        received
12345    1001     $9.99        paid
12345    1002     $19.98       shipped
```

and `ebpf_events_total{program="btf-uprobe",status=...}` in Grafana.

## Inspect the binary's BTF (the chapter's point)

On the VM (`dwarves` provides `pahole`, from Fedora repos):

```bash
[vm]$ sudo dnf install -y dwarves
[vm]$ pahole -J /home/fedora/target-app          # DWARF -> .BTF section
[vm]$ bpftool btf dump file /home/fedora/target-app | grep -iA4 order
```

That BTF dump is exactly the type information you'd use to generate the
`Order` mirror when probing a binary whose source you don't have.

## ⚠ Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on
the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, attaches
the uprobe on `process_order`, and reads the `Order` struct as
described. Full user-space CO-RE relocation is newer and less turnkey
than kernel CO-RE — the robust path shown here is the shared/generated
`#[repr(C)]` mirror. Attach targets and struct offsets can be
kernel- and compiler-version-specific.
