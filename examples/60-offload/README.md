# 60 · Offload: running eBPF off the host CPU

XDP attaches in three modes: **generic** and **native** run on the host CPU;
**offload** runs on the NIC itself. This example asks for offload first and
reports what the lab NIC actually supports.

## Reality check

- **True HW offload** (eBPF JITed into the NIC processor) needs narrow hardware
  — Netronome/Corigine **NFP**. Most NICs, including the lab's **virtio**, don't
  do it; BlueField-2 does only *driver*-mode XDP.
- **DPUs** (BlueField ARM Linux) are the practical answer: run **ordinary Aya**
  there by cross-compiling (`aarch64-unknown-linux-gnu`) — no code changes.
- **FPGA** (hXDP/eHDL) and **GPU/AI** (NVIDIA Morpheus) are the frontier.

## Pieces

- `offload-ebpf` — an `XDP_PASS` packet counter.
- `offload` — attaches asking `HW_MODE` → `DRV_MODE` → `SKB_MODE`, prints the
  engaged mode; exports `ebpf_offload_packets_total`.

## Run it

```bash
./demo.sh          # reports the engaged mode on the lab NIC (expect DRV/SKB)
./demo.sh build
```

## Cross-check

```bash
ip -d link show dev <iface>     # xdpdrv (native) vs xdpoffload
sudo bpftool prog show          # 'offloaded_to' only on offload-capable HW
sudo bpftool net show
```

## Verification status

**Verified (partial) — Fedora 44, kernel 7.1.3.** Built on the host and run on
the lab VM: the XDP program builds, loads, and attaches, and the loader reports
the engaged mode — `DRV`/`SKB` on the virtio NIC, never `HW`. Verified only in
SKB/DRV mode on virtio; the true HW-offload, DPU, FPGA, and GPU/AI claims
describe external hardware and projects this lab cannot exercise, so only
mode-selection is runnable here.
