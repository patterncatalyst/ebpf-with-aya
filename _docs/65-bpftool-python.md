---
title: "bpftool from Python: inventory and audit"
order: 65
part: Field guide
description: "bpftool was our ground truth all book — prog show, map dump, link show, struct_ops register. It speaks JSON, so a small Python wrapper turns it into real tools: an inventory of every loaded program, a runtime 'top' of which BPF costs the most CPU, map dumps, attachment audits, and a kernel-feature summary. Several working examples you can run on any box."
duration: 35 minutes
---

Where bpftrace (Chapter 64) *asks the kernel questions*, **bpftool** *inspects
and manages what's already loaded* — and it was the cross-check in nearly every
chapter: `prog show` to confirm our program loaded, `map dump` to verify a
counter, `link show` for the attachment, `struct_ops register` and `iter pin` to
drive features Aya couldn't yet author. Like bpftrace it speaks **JSON** (`-j`),
so the same subprocess-and-parse pattern turns it into proper tools: a host BPF
**inventory**, a runtime **top**, map dumps, and an attachment **audit**. This
chapter is several working examples, not one.

The code is in `examples/65-bpftool-python/`. `./demo.sh` runs the wrapper's
commands against a live program; the `README.md` has the details.

{% include excalidraw.html
   file="bpftool-python"
   alt="Drive bpftool from Python: inventory, inspect, and audit the BPF on a host. A Python tool using subprocess and json.loads issues a subcommand to bpftool -j (prog/map/link show, map dump, feature probe, net show), which queries the kernel's loaded programs, maps, links, and BTF and returns a JSON inventory. The tool parses it into progs, top by run_time, maps, dump, links, net, features, and audit tables. The swiss-army knife we cross-checked with all book, now scriptable; the run_time top needs sysctl kernel.bpf_stats_enabled=1."
   caption="Figure 65.1 — bpftool returns a JSON inventory of loaded BPF; Python turns it into tools" %}

## What bpftool sees

Every loaded BPF object has a kernel **id**, and bpftool enumerates them:
`prog show`, `map show`, `link show`, `btf show`. With `-j` each returns a JSON
array you can parse directly. A program object carries far more than its name:

```json
{ "id": 10, "type": "xdp", "name": "xdp_drop", "tag": "005a3d…",
  "gpl_compatible": true, "run_time_ns": 81632, "run_cnt": 10,
  "loaded_at": 1506715860, "bytes_xlated": 528, "jited": true,
  "bytes_jited": 370, "bytes_memlock": 4096, "map_ids": [10],
  "pids": [{ "pid": 1, "comm": "systemd" }] }
```

Three fields make the tools below worth building: **`map_ids`** ties a program
to the maps it uses; **`pids`** (since Linux 5.8) names the *processes holding
the program open* — who owns it; and **`run_time_ns`/`run_cnt`** give its total
CPU cost. That last pair is **zero unless you enable stats**, because collecting
them has a small per-run cost: `sysctl -w kernel.bpf_stats_enabled=1` turns them
on (Linux 5.1+).

## The wrapper: eight working commands

`examples/65-bpftool-python/bpftool_tool.py` is dependency-free and wraps
`bpftool -j` behind one helper, then exposes eight commands — each a useful tool
in its own right:

| Command | What it does | Built on |
|---|---|---|
| `progs` | host BPF inventory: id, type, name, JIT, memlock, maps, and the **processes holding** each program | `prog show` |
| `top` | programs by **avg ns/run** (`run_time_ns/run_cnt`) — which BPF costs the most CPU; warns if stats are off (`--enable-stats`) | `prog show` |
| `maps` | every map: id, type, name, key/value sizes, max entries, memlock | `map show` |
| `dump <name\|id>` | a map's contents as JSON — the `map dump` cross-check, scripted | `map dump` |
| `links` | links (attachments) and the program each drives (the Chapter 59 view) | `link show` |
| `net` | XDP/tc attachments per interface (the Chapter 60 cross-check) | `net show` |
| `features` | which program and map types this kernel supports | `feature probe` |
| `audit` | every loaded program with its **holders** and **attachments**, joined | `prog show` + `link show` |

The `top` and `audit` rows are the ones worth dwelling on: `top` answers "is our
probe expensive?" with the kernel's own numbers, and `audit` answers "what's
running and who put it there?" — the questions that matter in production. The
helper at the centre is four lines:

```python
def bpftool(*args):
    out = subprocess.run(["bpftool", "-j", *args], capture_output=True, text=True)
    if out.returncode != 0:
        raise RuntimeError(out.stderr.strip())
    return json.loads(out.stdout or "[]")
```

Everything else is shaping that JSON into a table or a join. The `audit` command
is the one to study: it builds a `prog_id → [link types]` map from `link show`,
then walks `prog show` printing each program's holders and attachments — exactly
the kind of cross-object correlation that's painful by eye and trivial in twenty
lines of Python over JSON.

## Why this matters operationally

bpftool is read-mostly truth. When an Aya program misbehaves, `progs`/`maps`
confirm what's actually loaded versus what you think you deployed; `dump`
verifies a counter independently of your loader's own reading; `top` answers
"is our probe expensive?" with the kernel's own numbers; and `audit` answers
"what's running and who put it there?" — the question that matters when you
inherit a host or chase a regression. Wrapping it in Python means these become
*repeatable* checks you can run across a fleet, diff over time, or wire into CI,
rather than commands you retype and eyeball.

## Build, deploy, observe

```bash
cd examples/65-bpftool-python && ./demo.sh
```

So there's something to inventory, the demo first starts a throwaway bpftrace
probe in the background (a loaded program with a map), then runs `progs`,
`maps`, `links`, `audit`, and `features` against it — and `top` with stats
enabled so you see real `run_cnt`/ns figures — before cleaning up. It's a
terminal tool, so no Grafana panel; the output *is* the inventory.

## Cross-check

```bash
[vm]$ sudo bpftool -j prog show | jq '.[].name'        # the raw inventory the tool parses
[vm]$ sudo bpftool prog show                            # human form, to compare
[vm]$ sudo sysctl kernel.bpf_stats_enabled               # is run_time being collected?
```

Running raw `bpftool -j prog show` and seeing the same ids your `progs` table
lists is the cross-check that the wrapper is a thin, faithful layer over the tool
you already trust.

## What you learned

- **bpftool** enumerates every loaded BPF object (`prog`/`map`/`link`/`btf
  show`) and emits **JSON** with `-j`; program objects expose `map_ids`, holder
  **`pids`** (5.8+), and **`run_time_ns`/`run_cnt`** (needs
  `kernel.bpf_stats_enabled=1`).
- A small Python wrapper turns that into tools: an **inventory** (`progs`/
  `maps`), a runtime **`top`**, map **`dump`**, attachment **`audit`**, and a
  feature summary — the cross-checks of the whole book, made repeatable.
- bpftool is **read-mostly ground truth**: it confirms what's actually loaded,
  what it costs, and who owns it — independent of your loader's own view.

Next, Chapter 66 tours the **BCC tools** — the ready-made `*-snoop`/`*-latency`
utilities — and drives them from Python, closing the field guide.

---

*Verification status: <span class="status status--verified">verified — Fedora 44, kernel 7.1.3</span>.
Built and run on the lab VM (Fedora 44, kernel 7.1.3-200.fc44): builds, loads, and attaches cleanly and runs without error. Confirmed on this kernel — attach targets and struct offsets can be version-specific.*
