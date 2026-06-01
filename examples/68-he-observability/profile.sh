#!/usr/bin/env bash
# examples/68-he-observability/profile.sh
# On-CPU profile the HE workload on the VM and show which stacks dominate — the
# NTT / polynomial-multiplication routines under he_compute. This is the "why"
# behind the latency histogram: the heatmap says compute is slow; the profile
# says the time is in the transform. It reads stack ADDRESSES, never operands.
#
# Usage: ./profile.sh [seconds]   (default 10)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"; SECS="${1:-10}"
IP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"
echo "━━ profiling he-workload on $VM for ${SECS}s (folded stacks; NTT should dominate he_compute)"
# -f folds stacks one-per-line ("a;b;c count") — ready to pipe into a flamegraph.
$SSH "sudo /usr/share/bcc/tools/profile -p \$(pgrep -n he-workload) -f ${SECS}" | tee /tmp/he-workload.folded
echo
echo "  wrote folded stacks to /tmp/he-workload.folded (on this host)"
echo "  flamegraph:  flamegraph.pl /tmp/he-workload.folded > he-workload.svg"
echo "  Grafana panel: point Grafana Alloy's eBPF profiler (or the Pyroscope eBPF"
echo "  profiler) at he-workload and view the flamegraph in the bundled Pyroscope —"
echo "  the NTT subtree sits under the he_compute branch."
