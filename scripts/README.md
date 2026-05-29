# scripts/

Tooling for the lab and the examples.

## `lab/` — the virtual-machine lab

| Script | Purpose |
|--------|---------|
| `provision-vm.sh <name>` | Provision a Fedora 44 KVM guest via cloud-init (`ebpf-target`, `ebpf-peer`). Set `BASE_IMG` to the current Fedora Cloud Base filename first. |
| `vm-ip.sh <name>` | Print the guest's leased IPv4. |
| `deploy-to-target.sh <name> <binary> [-- args]` | `scp` a built binary to the guest and run it under `sudo`. |
| `destroy-vm.sh <name>` | Tear the guest down, including its disks. |
| `cloud-init/` | `user-data.tmpl` + `meta-data` seed; installs kernel tooling from Fedora repos and checks BTF. |

Env overrides for `provision-vm.sh`: `VCPUS`, `RAM_MB`, `DISK_GB`,
`NETWORK`, `SSH_PUBKEY`.

## `lib/_helpers.sh`

Sourced by demo scripts: colors (`step`/`pass`/`fail`/`info`),
`repo_root`, `cleanup_container`, `wait_for_http` (uses `127.0.0.1`).

## `test-all-examples.sh`

Runs every `examples/*/demo.sh` and tallies pass/fail. Examples needing
the VM or the stack will fail fast if those aren't up.

All scripts assume a Fedora 44 host with libvirt + rootless Podman (see
Chapter 1). Per project policy, kernel tooling installed by these
scripts comes only from Fedora/Red Hat repositories.
