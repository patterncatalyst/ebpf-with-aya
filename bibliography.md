---
title: "Bibliography"
layout: plan
permalink: /bibliography/
description: "The reference works and primary documentation this tutorial's eBPF and Aya coverage draws on."
---

This tutorial teaches eBPF observation from user space with Aya (Rust). The works
below are the field's standard references and the authoritative documentation it
follows. They are reference works, not sources: no text or code from any of them
is reproduced here.

<table class="bib bib--books">
  <colgroup><col style="width:62%"><col style="width:24%"><col style="width:14%"></colgroup>
  <thead><tr><th>Title</th><th>Author(s)</th><th>Year</th></tr></thead>
  <tbody>
    <tr><td><a href="https://www.oreilly.com/library/view/learning-ebpf/9781098135119/" rel="noopener"><em>Learning eBPF</em></a><br><small>O'Reilly. What eBPF is, how programs attach, and what they can observe</small></td><td>Liz Rice</td><td>2023</td></tr>
    <tr><td><a href="https://www.brendangregg.com/bpf-performance-tools-book.html" rel="noopener"><em>BPF Performance Tools</em></a><br><small>Addison-Wesley. bcc/bpftrace tools for Linux observability and performance</small></td><td>Brendan Gregg</td><td>2019</td></tr>
    <tr><td><a href="https://doc.rust-lang.org/book/" rel="noopener"><em>The Rust Programming Language</em></a><br><small>No Starch Press. The Rust language the Aya user-space programs are written in</small></td><td>Steve Klabnik, Carol Nichols</td><td>2023</td></tr>
  </tbody>
</table>

## Primary references

- [Aya documentation](https://aya-rs.dev/): the Rust eBPF library this tutorial uses from user space.
- [Linux kernel BPF documentation](https://docs.kernel.org/bpf/): program types, maps, BTF, and the verifier.
- [bpftrace](https://github.com/bpftrace/bpftrace) and [bcc](https://github.com/iovisor/bcc): the reference tracing tools the examples compare against.

Nothing in these chapters or the runnable examples copies text or code from these
works.
