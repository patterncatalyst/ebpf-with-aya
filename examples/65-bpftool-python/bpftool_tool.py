#!/usr/bin/env python3
"""Drive bpftool from Python via its JSON output (-j). Inventory and audit the
BPF objects loaded on a host: programs, maps, links, attachments, a runtime
'top', and kernel features. Standard library only; run with sudo.

Commands:
  progs      every loaded program (id/type/name/jit/memlock/maps/holders)
  top        programs by avg ns/run     (needs kernel.bpf_stats_enabled=1)
  maps       every map (sizes, entries, memlock)
  dump NAME  a map's contents as JSON   (NAME may be a name or numeric id)
  links      links (attachments) and the program each drives
  net        XDP/tc attachments per interface
  features   supported program & map types (from feature probe)
  audit      every program with its holders and attachments (joined)
"""
import argparse
import json
import subprocess
import sys


def bpftool(*args):
    out = subprocess.run(["bpftool", "-j", *args], capture_output=True, text=True)
    if out.returncode != 0:
        raise RuntimeError(out.stderr.strip() or f"bpftool {' '.join(args)} failed")
    return json.loads(out.stdout or "[]")


def cmd_progs(a):
    progs = bpftool("prog", "show")
    print(f"{'ID':>5} {'TYPE':<16} {'NAME':<20} {'JIT':<4} {'MEMLOCK':>9} {'MAPS':<10} HOLDERS")
    for p in progs:
        holders = ",".join(x.get("comm", "?") for x in p.get("pids", [])) or "-"
        maps = ",".join(str(m) for m in p.get("map_ids", [])) or "-"
        print(f"{p.get('id'):>5} {p.get('type',''):<16} {p.get('name','-'):<20} "
              f"{'y' if p.get('jited') else 'n':<4} {p.get('bytes_memlock',0):>9} {maps:<10} {holders}")


def cmd_top(a):
    if a.enable_stats:
        subprocess.run(["sysctl", "-w", "kernel.bpf_stats_enabled=1"], capture_output=True)
    progs = bpftool("prog", "show")
    rows = []
    for p in progs:
        rt, rc = p.get("run_time_ns", 0), p.get("run_cnt", 0)
        rows.append(((rt / rc) if rc else 0.0, rt, rc, p))
    if not any(r[2] for r in rows):
        print("run stats are zero — enable with: sudo sysctl -w kernel.bpf_stats_enabled=1")
        print("(or: sudo ./bpftool_tool.py top --enable-stats, then generate activity)")
    rows.sort(key=lambda r: (r[0], r[1], r[2]), reverse=True)  # numeric fields only; the trailing dict isn't comparable
    print(f"{'ID':>5} {'TYPE':<16} {'NAME':<20} {'AVG ns/run':>12} {'RUN_CNT':>14}")
    for avg, rt, rc, p in rows[:a.top]:
        print(f"{p.get('id'):>5} {p.get('type',''):<16} {p.get('name','-'):<20} {avg:>12.0f} {rc:>14}")


def cmd_maps(a):
    maps = bpftool("map", "show")
    print(f"{'ID':>5} {'TYPE':<18} {'NAME':<20} {'KEY':>5} {'VAL':>7} {'MAX':>9} {'MEMLOCK':>9}")
    for m in maps:
        print(f"{m.get('id'):>5} {m.get('type',''):<18} {m.get('name','-'):<20} "
              f"{m.get('bytes_key',0):>5} {m.get('bytes_value',0):>7} "
              f"{m.get('max_entries',0):>9} {m.get('bytes_memlock',0):>9}")


def cmd_dump(a):
    if not a.target:
        print("usage: dump <map-name-or-id>", file=sys.stderr); sys.exit(2)
    sel = ["id", a.target] if a.target.isdigit() else ["name", a.target]
    print(json.dumps(bpftool("map", "dump", *sel), indent=2)[:4000])


def cmd_links(a):
    links = bpftool("link", "show")
    print(f"{'ID':>5} {'TYPE':<16} {'PROG':>6}")
    for l in links:
        print(f"{l.get('id'):>5} {l.get('type',''):<16} {str(l.get('prog_id','-')):>6}")


def cmd_net(a):
    print(json.dumps(bpftool("net", "show"), indent=2)[:4000])


def cmd_features(a):
    f = bpftool("feature", "probe")
    f = f[0] if isinstance(f, list) and f else f
    def supported(d):
        return sorted(k for k, v in d.items() if v is True) if isinstance(d, dict) else []
    pt = (f or {}).get("program_types", {})
    mt = (f or {}).get("map_types", {})
    if pt: print("supported program types:", ", ".join(supported(pt)))
    if mt: print("supported map types:", ", ".join(supported(mt)))
    if not pt and not mt:
        print("feature-probe JSON shape differs on this version; raw keys:",
              ", ".join((f or {}).keys()))


def cmd_audit(a):
    progs = bpftool("prog", "show")
    links = bpftool("link", "show")
    by_prog = {}
    for l in links:
        by_prog.setdefault(l.get("prog_id"), []).append(l.get("type", "?"))
    print("loaded BPF programs — holders and attachments:")
    for p in progs:
        holders = ",".join(x.get("comm", "?") for x in p.get("pids", [])) or "-"
        atts = ",".join(by_prog.get(p.get("id"), [])) or "(none / legacy attach)"
        print(f"  [{p.get('id')}] {p.get('type','')}/{p.get('name','-'):<20} "
              f"holders={holders}  links={atts}")


CMDS = {"progs": cmd_progs, "top": cmd_top, "maps": cmd_maps, "dump": cmd_dump,
        "links": cmd_links, "net": cmd_net, "features": cmd_features, "audit": cmd_audit}


def main():
    ap = argparse.ArgumentParser(description="drive bpftool from Python (JSON)")
    ap.add_argument("command", choices=list(CMDS))
    ap.add_argument("target", nargs="?", help="map name/id for 'dump'")
    ap.add_argument("--top", type=int, default=15)
    ap.add_argument("--enable-stats", action="store_true",
                    help="(top) set kernel.bpf_stats_enabled=1 first")
    a = ap.parse_args()
    try:
        CMDS[a.command](a)
    except RuntimeError as e:
        print(f"error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
