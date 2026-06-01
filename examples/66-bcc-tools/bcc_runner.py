#!/usr/bin/env python3
"""Run a BCC tool (from bcc-tools) and summarize its output from Python. BCC
tools print columnar text (not JSON), so we resolve the tool across distro
layouts, run it for a duration, parse the columns we know into a top-N summary,
and for tools we don't parse we print their own (already-summarized) output.
Standard library only; run with sudo.

Examples:
  sudo ./bcc_runner.py execsnoop
  sudo ./bcc_runner.py opensnoop --duration 10
  sudo ./bcc_runner.py tcpconnect
  sudo ./bcc_runner.py biolatency 5 1        # extra args pass through to the tool
"""
import argparse
import collections
import os
import select
import shutil
import signal
import subprocess
import sys
import time

SEARCH = ["/usr/share/bcc/tools", "/sbin", "/usr/sbin", "/usr/bin"]


def resolve(tool):
    if os.path.sep in tool and os.path.exists(tool):
        return tool
    for d in SEARCH:
        p = os.path.join(d, tool)
        if os.path.exists(p):
            return p
    return shutil.which(tool) or shutil.which(tool + "-bpfcc")


# --- per-tool column parsers: feed a line, update a Counter ---
def p_execsnoop(state, line):                 # PCOMM PID PPID RET ARGS
    f = line.split()
    if len(f) >= 4 and f[1].isdigit():
        state[f[0]] += 1


def p_opensnoop(state, line):                 # PID COMM FD ERR PATH...
    f = line.split()
    if len(f) >= 5 and f[0].isdigit():
        state[f[-1]] += 1


def p_tcpconnect(state, line):                # PID COMM IP SADDR DADDR DPORT
    f = line.split()
    if len(f) >= 6 and f[0].isdigit():
        state[f[-2] + ":" + f[-1]] += 1


PARSERS = {
    "execsnoop": (p_execsnoop, "command", "execs"),
    "opensnoop": (p_opensnoop, "path", "opens"),
    "tcpconnect": (p_tcpconnect, "dest", "connects"),
}


def main():
    ap = argparse.ArgumentParser(description="run + summarize a BCC tool")
    ap.add_argument("tool", help="bcc tool name, e.g. execsnoop / biolatency / runqlat")
    ap.add_argument("args", nargs="*", help="extra args passed through to the tool")
    ap.add_argument("--duration", type=int, default=8)
    ap.add_argument("--top", type=int, default=15)
    a = ap.parse_args()

    path = resolve(a.tool)
    if not path:
        print(f"could not find '{a.tool}'. Install bcc-tools; looked in "
              f"{SEARCH} and $PATH (with/without -bpfcc).", file=sys.stderr)
        sys.exit(1)

    print(f"running {path} {' '.join(a.args)}  (for {a.duration}s)")
    proc = subprocess.Popen([path, *a.args], stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True)
    parser = PARSERS.get(a.tool)
    state = collections.Counter()
    captured = []
    deadline = time.time() + a.duration

    def consume(line):
        line = line.rstrip("\n")
        if parser:
            parser[0](state, line)
        else:
            captured.append(line)

    try:
        while time.time() < deadline:
            r, _, _ = select.select([proc.stdout], [], [], 0.3)
            if r:
                line = proc.stdout.readline()
                if not line:
                    break
                consume(line)
    except KeyboardInterrupt:
        pass
    finally:
        proc.send_signal(signal.SIGINT)           # let summarizing tools flush
        try:
            rest, _ = proc.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            rest = ""
        for line in (rest or "").splitlines():
            consume(line)

    if parser:
        _, kname, vname = parser
        print(f"\n== top {a.top} by {vname} ({kname}) ==")
        for k, c in state.most_common(a.top):
            print(f"  {str(k):<44} {c}")
        if not state:
            print("  (no rows — generate some activity while it runs)")
    else:
        print("\n".join(captured[-40:]) or "(no output captured)")


if __name__ == "__main__":
    main()
