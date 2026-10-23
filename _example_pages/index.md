---
title: "Examples"
layout: plan
permalink: /examples/
description: "Every runnable example in this repo, mapped to the chapter it supports."
---

Each hands-on chapter has a matching runnable example under
[`examples/`](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples).
Each pairs an Aya (Rust) user-space loader with its eBPF program and a verify script.

| Example | What it shows |
|---|---|
| [03-observability-stack](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/03-observability-stack) | Observability stack |
| [06-hello-world](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/06-hello-world) | Hello world |
| [07-kprobe-unlink](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/07-kprobe-unlink) | Kprobe unlink |
| [08-fentry-unlink](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/08-fentry-unlink) | Fentry unlink |
| [09-opensnoop](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/09-opensnoop) | Opensnoop |
| [10-sigsnoop](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/10-sigsnoop) | Sigsnoop |
| [11-execsnoop](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/11-execsnoop) | Execsnoop |
| [12-exitsnoop](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/12-exitsnoop) | Exitsnoop |
| [13-bashreadline](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/13-bashreadline) | Bashreadline |
| [14-uprobe-rust](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/14-uprobe-rust) | Uprobe rust |
| [15-btf-uprobe](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/15-btf-uprobe) | Btf uprobe |
| [16-container-targets](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/16-container-targets) | Container targets |
| [17-sslsniff](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/17-sslsniff) | Sslsniff |
| [18-funclatency](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/18-funclatency) | Funclatency |
| [19-goroutine-states](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/19-goroutine-states) | Goroutine states |
| [20-javagc](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/20-javagc) | Javagc |
| [21-runqlat](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/21-runqlat) | Runqlat |
| [22-hardirqs](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/22-hardirqs) | Hardirqs |
| [23-profile](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/23-profile) | Profile |
| [24-memleak](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/24-memleak) | Memleak |
| [25-biopattern](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/25-biopattern) | Biopattern |
| [26-energy](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/26-energy) | Energy |
| [27-tcpconnlat](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/27-tcpconnlat) | Tcpconnlat |
| [28-tcpstates](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/28-tcpstates) | Tcpstates |
| [29-http-l7](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/29-http-l7) | Http l7 |
| [30-sockops](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/30-sockops) | Sockops |
| [31-tc-classify](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/31-tc-classify) | Tc classify |
| [32-xdp-drop](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/32-xdp-drop) | Xdp drop |
| [33-xdp-capture](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/33-xdp-capture) | Xdp capture |
| [34-xdp-lb](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/34-xdp-lb) | Xdp lb |
| [35-xdp-test](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/35-xdp-test) | Xdp test |
| [36-tcx](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/36-tcx) | Tcx |
| [37-lsm-confine](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/37-lsm-confine) | Lsm confine |
| [38-signal-kill](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/38-signal-kill) | Signal kill |
| [39-pidhide](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/39-pidhide) | Pidhide |
| [40-lsm-fileprotect](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/40-lsm-fileprotect) | Lsm fileprotect |
| [41-sudoadd](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/41-sudoadd) | Sudoadd |
| [42-secsensor](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/42-secsensor) | Secsensor |
| [43-scx-simple](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/43-scx-simple) | Scx simple |
| [44-scx-nest](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/44-scx-nest) | Scx nest |
| [45-nginx-probe](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/45-nginx-probe) | Nginx probe |
| [46-three-signals](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/46-three-signals) | Three signals |
| [47-pg-probe](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/47-pg-probe) | Pg probe |
| [48-pin-demo](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/48-pin-demo) | Pin demo |
| [49-syscall-prog](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/49-syscall-prog) | Syscall prog |
| [50-user-rb](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/50-user-rb) | User rb |
| [51-userspace-ebpf](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/51-userspace-ebpf) | Userspace ebpf |
| [52-kfunc-task](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/52-kfunc-task) | Kfunc task |
| [53-bpf-token](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/53-bpf-token) | Bpf token |
| [54-bpf-timer](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/54-bpf-timer) | Bpf timer |
| [55-structops-cc](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/55-structops-cc) | Structops cc |
| [56-dynptr-arena](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/56-dynptr-arena) | Dynptr arena |
| [57-bpf-iter](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/57-bpf-iter) | Bpf iter |
| [58-core](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/58-core) | Core |
| [59-bpf-lifecycle](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/59-bpf-lifecycle) | Bpf lifecycle |
| [60-offload](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/60-offload) | Offload |
| [61-power](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/61-power) | Power |
| [62-correlation](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/62-correlation) | Correlation |
| [63-capstone](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/63-capstone) | Final project |
| [64-bpftrace-python](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/64-bpftrace-python) | Bpftrace python |
| [65-bpftool-python](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/65-bpftool-python) | Bpftool python |
| [66-bcc-tools](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/66-bcc-tools) | Bcc tools |
| [68-he-observability](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/68-he-observability) | He observability |
| [69-pcp-ebpf](https://github.com/{{ site.github_username }}/{{ site.github_repo }}/tree/main/examples/69-pcp-ebpf) | Pcp ebpf |
