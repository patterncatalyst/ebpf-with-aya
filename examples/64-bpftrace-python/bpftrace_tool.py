#!/usr/bin/env python3
"""Drive bpftrace from Python. Run a .bt program (or an inline one-liner) with
-f json (NDJSON output) and turn its event stream into something useful:
- printf events  -> streamed as-is (snoop-style tools)
- map events     -> a top-N table, or a histogram bar chart (lhist buckets)
Standard library only. Run with sudo (bpftrace needs privileges).

Examples:
  sudo ./bpftrace_tool.py --list
  sudo ./bpftrace_tool.py --program programs/syscount.bt --duration 8
  sudo ./bpftrace_tool.py --program programs/execsnoop.bt
  sudo ./bpftrace_tool.py -e 'tracepoint:syscalls:sys_enter_openat { @[comm]=count(); }
                              interval:s:1 { print(@); clear(@); }'
"""
import argparse
import glob
import json
import os
import signal
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
PROGDIR = os.path.join(HERE, "programs")


def first_comment(path):
    try:
        with open(path) as f:
            for line in f:
                s = line.strip()
                if s.startswith("//"):
                    return s.lstrip("/ ").strip()
                if s:
                    return ""
    except OSError:
        pass
    return ""


def list_programs():
    print("available programs (in programs/):")
    for p in sorted(glob.glob(os.path.join(PROGDIR, "*.bt"))):
        print(f"  {os.path.basename(p):<16} {first_comment(p)}")


def render_map(name, data, topn):
    print(f"\n== {name} (top {topn}) ==")
    if isinstance(data, dict):                       # @[key] = count()/sum()/...
        rows = sorted(
            data.items(),
            key=lambda kv: kv[1] if isinstance(kv[1], (int, float)) else 0,
            reverse=True,
        )
        for k, v in rows[:topn]:
            print(f"  {str(k):<24} {v}")
    elif isinstance(data, list):                     # lhist/hist buckets
        mx = max((b.get("count", 0) for b in data), default=1) or 1
        for b in data:
            lo, hi, c = b.get("min", ""), b.get("max", "inf"), b.get("count", 0)
            bar = "#" * int(40 * c / mx)
            print(f"  [{str(lo):>7}, {str(hi):>7}) {c:>7} {bar}")


def run(cmd, duration, topn):
    print(f"running: {' '.join(cmd[:4])} …  (for {duration}s, Ctrl-C to stop)")
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    deadline = time.time() + duration
    try:
        for line in proc.stdout:
            line = line.strip()
            if not line:
                continue
            try:
                evt = json.loads(line)               # NDJSON: one object per line
            except json.JSONDecodeError:
                continue
            t, d = evt.get("type"), evt.get("data")
            if t == "attached_probes":
                print(f"attached {d.get('probes')} probe(s)")
            elif t == "printf":
                sys.stdout.write(d)                  # snoop-style stream
                sys.stdout.flush()
            elif t in ("map", "hist", "stats"):
                if isinstance(d, dict):
                    for name, val in d.items():
                        render_map(name, val, topn)
            elif t == "value":
                print(f"value: {d}")
            if time.time() > deadline:
                break
    except KeyboardInterrupt:
        pass
    finally:
        proc.send_signal(signal.SIGINT)
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()
        err = proc.stderr.read() if proc.stderr else ""
        if err.strip():
            print(f"[bpftrace stderr] {err.strip()}", file=sys.stderr)
    # --- OTel export hook: POST the latest counts to your OTLP endpoint here. ---


def main():
    ap = argparse.ArgumentParser(description="drive a bpftrace program from Python")
    ap.add_argument("--program", help="path to a .bt file (default: programs/syscount.bt)")
    ap.add_argument("-e", "--oneliner", help="inline bpftrace program text")
    ap.add_argument("--list", action="store_true", help="list bundled programs and exit")
    ap.add_argument("--duration", type=int, default=8, help="seconds to run")
    ap.add_argument("--top", type=int, default=15)
    args = ap.parse_args()

    if args.list:
        list_programs()
        return
    if args.oneliner:
        cmd = ["bpftrace", "-q", "-f", "json", "-e", args.oneliner]
    else:
        prog = args.program or os.path.join(PROGDIR, "syscount.bt")
        cmd = ["bpftrace", "-q", "-f", "json", prog]
    run(cmd, args.duration, args.top)


if __name__ == "__main__":
    main()
