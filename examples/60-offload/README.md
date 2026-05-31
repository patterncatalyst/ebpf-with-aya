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

**Unverified.** Confirm the XDP program attaches and the loader reports the
engaged mode (expect `DRV`/`SKB` on virtio, not `HW`). All HW-offload/DPU/FPGA/
GPU claims describe external hardware/projects the lab can't exercise; only
mode-selection is runnable here.
