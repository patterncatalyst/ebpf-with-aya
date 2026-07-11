# 35 · Testing eBPF with BPF_PROG_TEST_RUN

A unit-test harness for an XDP program. It loads the program, feeds it
synthetic packets via `BPF_PROG_TEST_RUN`, asserts the verdict for each,
and checks a map side-effect — no NIC, no peer VM, no live traffic.

## What it does

- `xdp-test-ebpf` is the program under test: drop ICMP, pass everything
  else, count packets per protocol in `PKTS`.
- `xdp-test` (the harness) loads it, then for each case builds a packet
  (`ICMP -> DROP`, `TCP -> PASS`, `ARP -> PASS`), runs it through
  `BPF_PROG_TEST_RUN`, and compares the returned verdict to the expected
  one. It then asserts `PKTS[icmp] >= 1` to show the map really moved.
- Prints a got/want/result table and exits non-zero if any case fails — so
  it drops into CI.

## Run it

```bash
./demo.sh          # build + deploy to $VM + run under sudo
./demo.sh build    # just build on the host
```

Only the target VM is needed (the syscall requires `CAP_BPF`); no peer and
no traffic generation.

## Verify on the target

```bash
sudo bpftool prog list                                            # find the prog id
sudo bpftool prog run id <ID> data_in pkt.bin data_out out.bin repeat 1
```

`bpftool prog run` is the same `BPF_PROG_TEST_RUN`; the same verdict from
both confirms the harness.

## Verification status

**Verified — Fedora 44, kernel 7.1.3.** Built on the host and run on the
lab VM (Fedora 44, kernel 7.1.3-200.fc44): the harness builds, loads the
program, issues `BPF_PROG_TEST_RUN` via the syscall wrapper, and asserts
each case's verdict along with the `PKTS` map side-effect as described. The
`bpf_attr` test layout, the syscall command number, and XDP test-input
handling can be kernel-version-specific.
