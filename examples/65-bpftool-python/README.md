# 65 · bpftool from Python: inventory and audit

bpftool inspects what's already loaded — our cross-check all book. With `-j` it
emits JSON, so a Python wrapper turns it into repeatable tools.

## `bpftool_tool.py` commands (each a working example)

- `progs` — host BPF inventory (id/type/name/jit/memlock/maps/holders)
- `top` — programs by avg ns/run (needs `sysctl kernel.bpf_stats_enabled=1`; use `--enable-stats`)
- `maps` — every map (sizes, entries, memlock)
- `dump <name|id>` — a map's contents as JSON
- `links` — links (attachments) and their program
- `net` — XDP/tc attachments per interface
- `features` — supported program/map types (from `feature probe`)
- `audit` — every program with its holders + attachments (joined across show calls)

## Run it

```bash
./demo.sh                       # inventory/audit against a throwaway probe on the VM
# on the VM (needs sudo):
sudo python3 bpftool_tool.py progs
sudo python3 bpftool_tool.py top --enable-stats
sudo python3 bpftool_tool.py audit
sudo python3 bpftool_tool.py dump <map-name>
```

## Verification status

**Unverified.** Confirm `bpftool -j` field names for your version
(`bpftool version`); that `feature probe` nests `program_types`/`map_types` as
`features` expects (shapes vary — falls back to raw keys); and that
`kernel.bpf_stats_enabled=1` populates `run_time_ns`/`run_cnt`.
