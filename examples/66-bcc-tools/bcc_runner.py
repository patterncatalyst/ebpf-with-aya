#!/usr/bin/env python3
"""Run a BCC tool (from bcc-tools) and summarize its output from Python. BCC
tools print columnar text (not JSON), so we resolve the tool across distro
layouts, run it for a duration, parse the columns we know into a top-N summary,
and for tools we don't parse we print their own (already-summarized) output.
Standard library only; run with sudo.

  sudo ./bcc_runner.py --list
  sudo ./bcc_runner.py execsnoop
  sudo ./bcc_runner.py tcplife --duration 12
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


# --- column parsers: each gets (Counter, line) and tallies one key -----------
def _first_pid(f):
    for i, tok in enumerate(f):
        if tok.isdigit():
            return i
    return -1


def p_by_comm(state, line):                   # execsnoop: PCOMM PID PPID RET ARGS
    f = line.split()
    if len(f) >= 4 and f[1].isdigit():
        state[f[0]] += 1


def p_last_path(state, line):                 # opensnoop/statsnoop: PID COMM FD ERR PATH...
    f = line.split()
    if len(f) >= 5 and f[0].isdigit():
        state[f[-1]] += 1


def p_tcp_dest(state, line):                  # tcpconnect: PID COMM IP SADDR DADDR DPORT
    f = line.split()
    if len(f) >= 6 and f[0].isdigit():
        state[f[-2] + ":" + f[-1]] += 1


def p_kill(state, line):                      # killsnoop: [TIME] PID COMM SIG TPID RESULT
    f = line.split()
    i = _first_pid(f)
    if i >= 0 and len(f) > i + 1:
        state[f[i + 1]] += 1                  # tally by signalling command


def p_tcpaccept(state, line):                 # tcpaccept: PID COMM IP RADDR RPORT LADDR LPORT
    f = line.split()
    if len(f) >= 5 and f[0].isdigit():
        state[f[3]] += 1                      # remote address


def p_tcplife(state, line):                   # tcplife: PID COMM LADDR LPORT RADDR RPORT ...
    f = line.split()
    if len(f) >= 6 and f[0].isdigit():
        state[f[4] + ":" + f[5]] += 1


def p_syscount(state, line):                  # syscount table rows: SYSCALL COUNT
    f = line.split()
    if len(f) == 2 and f[1].isdigit() and not f[0].isdigit():
        state[f[0]] += int(f[1])


# tool -> (parser, key-name, value-name)
PARSERS = {
    "execsnoop": (p_by_comm, "command", "execs"),
    "opensnoop": (p_last_path, "path", "opens"),
    "statsnoop": (p_last_path, "path", "stats"),
    "tcpconnect": (p_tcp_dest, "dest", "connects"),
    "tcpaccept": (p_tcpaccept, "remote", "accepts"),
    "tcplife": (p_tcplife, "remote", "sessions"),
    "killsnoop": (p_kill, "killer", "signals"),
    "syscount": (p_syscount, "syscall", "calls"),
}
CAPTURE_TOOLS = ["biolatency", "runqlat", "profile", "cachestat", "biotop",
                 "tcptop", "funccount", "funclatency"]


def main():
    ap = argparse.ArgumentParser(description="run + summarize a BCC tool")
    ap.add_argument("tool", nargs="?", help="bcc tool name, e.g. execsnoop / biolatency")
    ap.add_argument("args", nargs="*", help="extra args passed through to the tool")
    ap.add_argument("--list", action="store_true", help="list tools the wrapper summarizes")
    ap.add_argument("--duration", type=int, default=8)
    ap.add_argument("--top", type=int, default=15)
    a = ap.parse_args()

    if a.list or not a.tool:
        print("parsed into a top-N summary:")
        for t, (_, k, v) in PARSERS.items():
            print(f"  {t:<12} top by {v} ({k})")
        print("captured + printed as-is (tool already summarizes):")
        print("  " + " ".join(CAPTURE_TOOLS))
        print("any other tool also runs (output captured). examples:")
        print("  funccount 'vfs_*'   argdist -H 'r::vfs_read()'   trace 'do_sys_openat2'")
        return

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

    def consume(line):
        line = line.rstrip("\n")
        if parser:
            parser[0](state, line)
        else:
            captured.append(line)

    deadline = time.time() + a.duration
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
            print("  (no rows — generate activity while it runs, or the column "
                  "layout differs on your version)")
    else:
        print("\n".join(captured[-40:]) or "(no output captured)")


if __name__ == "__main__":
    main()
