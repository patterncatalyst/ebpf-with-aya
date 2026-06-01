#!/usr/bin/env python3
"""Drive bpftrace from Python. Run a .bt program with -f json (NDJSON output)
and turn its map events into a live top-N table / histogram. Standard library
only — no pip installs. Run with sudo (bpftrace needs privileges)."""
import argparse
import json
import os
import signal
import subprocess
import sys
import time


def render_map(name, data, topn):
    print(f"\n== {name} (top {topn}) ==")
    if isinstance(data, dict):                       # @[key] = count()
        rows = sorted(
            data.items(),
            key=lambda kv: kv[1] if isinstance(kv[1], (int, float)) else 0,
            reverse=True,
        )
        for k, v in rows[:topn]:
            print(f"  {str(k):<20} {v}")
    elif isinstance(data, list):                     # lhist/hist buckets
        mx = max((b.get("count", 0) for b in data), default=1) or 1
        for b in data:
            lo, hi, c = b.get("min", ""), b.get("max", "inf"), b.get("count", 0)
            bar = "#" * int(40 * c / mx)
            print(f"  [{str(lo):>6}, {str(hi):>6}) {c:>6} {bar}")


def main():
    ap = argparse.ArgumentParser(description="drive a bpftrace program from Python")
    here = os.path.dirname(os.path.abspath(__file__))
    ap.add_argument("--program", default=os.path.join(here, "programs", "syscount.bt"))
    ap.add_argument("--duration", type=int, default=8, help="seconds to run")
    ap.add_argument("--top", type=int, default=15)
    args = ap.parse_args()

    cmd = ["bpftrace", "-q", "-f", "json", args.program]
    print(f"running: {' '.join(cmd)}  (for {args.duration}s)")
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    deadline = time.time() + args.duration
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
                sys.stdout.write(d)
            elif t == "map":
                for name, val in d.items():
                    render_map(name, val, args.top)
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

    # --- OTel export hook -------------------------------------------------
    # To publish ebpf_* metrics to the Chapter 3 stack, POST the latest counts
    # to your OTLP endpoint here (or use the opentelemetry SDK). Omitted to keep
    # this wrapper dependency-free.


if __name__ == "__main__":
    main()
