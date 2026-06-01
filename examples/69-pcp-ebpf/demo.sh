#!/usr/bin/env bash
# examples/69-pcp-ebpf/demo.sh
# Stand up Performance Co-Pilot on the VM, enable the eBPF (CO-RE) PMDA, show the
# BPF metrics under bpf.*, record + replay a short pmlogger archive, and drop an
# OpenMetrics bridge so the book's ebpf_* metrics can flow into PCP too.
# All steps run on the target VM. Nothing here is destructive beyond installing
# PCP packages and enabling PMDAs.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)" && cd "$SCRIPT_DIR"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"; LAB="$REPO_ROOT/scripts/lab"
VM="${VM:-ebpf-target}"
# Prometheus endpoint carrying ebpf_* metrics to bridge into PCP (optional).
# Default: the otel-lgtm bundled Prometheus on the host gateway.
PROM_URL="${PROM_URL:-}"
c_step(){ echo -e "\033[0;36m━━ $*\033[0m"; }; c_ok(){ echo -e "\033[0;32m✓ $*\033[0m"; }
c_info(){ echo -e "\033[1;33m  $*\033[0m"; }

IP="$("$LAB/vm-ip.sh" "$VM")"
SSH="ssh -o StrictHostKeyChecking=accept-new fedora@$IP"

c_step "installing PCP + BPF/bpftrace PMDAs + grafana-pcp (Fedora repos)"
$SSH 'sudo dnf install -y pcp pcp-zeroconf pcp-pmda-bpf pcp-pmda-bpftrace grafana-pcp'
$SSH 'sudo systemctl enable --now pmcd pmlogger pmproxy'
c_ok "pmcd/pmlogger/pmproxy running"

c_step "enabling eBPF CO-RE modules in the BPF PMDA (runqlat, biolatency)"
# Flip enabled=true for a couple of modules, then (re)install the agent so pmcd
# picks them up. Module section names come from the shipped bpf.conf.
$SSH 'sudo python3 - <<PY
import re,io
p="/var/lib/pcp/pmdas/bpf/bpf.conf"
s=open(p).read()
for mod in ("runqlat","biolatency"):
    s=re.sub(r"(\["+mod+r"\][^\[]*?)enabled\s*=\s*\w+", r"\1enabled=true", s, flags=re.S) \
        if re.search(r"\["+mod+r"\]",s) else s
open(p,"w").write(s)
print("updated", p)
PY'
$SSH 'cd /var/lib/pcp/pmdas/bpf && sudo ./Install'
c_ok "BPF PMDA installed"

c_step "the eBPF metrics are now first-class PCP metrics under bpf.*"
$SSH 'pminfo bpf | head; echo; pminfo -f bpf.runqlat 2>/dev/null | head || true'
$SSH 'pmrep -p -t 1 -s 3 bpf.runqlat 2>/dev/null || true'

c_step "pmlogger archive: record 10s, then replay it (retrospective view)"
$SSH 'A=/tmp/pcp-ebpf-demo; rm -f $A.*; pmlogger -T 10s -c /dev/stdin $A <<CONF
log mandatory on 1 sec { bpf.runqlat }
CONF
echo "--- replay ---"; pmrep --archive $A bpf.runqlat 2>/dev/null | head || true'

c_step "optional: bridge the book's ebpf_* Prometheus metrics into PCP"
if [[ -n "$PROM_URL" ]]; then
  c_info "pointing pmdaopenmetrics at $PROM_URL"
  $SSH "sudo install -d /var/lib/pcp/pmdas/openmetrics/config.d && \
        echo '$PROM_URL' | sudo tee /var/lib/pcp/pmdas/openmetrics/config.d/ebpf-aya.url >/dev/null && \
        sudo dnf install -y pcp-pmda-openmetrics && \
        cd /var/lib/pcp/pmdas/openmetrics && sudo ./Install"
  $SSH 'pminfo openmetrics 2>/dev/null | head || true'
else
  c_info "set PROM_URL=http://<gateway>:9090/metrics (or a loader /metrics) to enable the bridge"
  c_info "see openmetrics-ebpf.url.example in this directory"
fi
c_ok "done — add the PCP data source in Grafana via grafana-pcp (see README)"
