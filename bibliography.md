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

| Title | Author(s) | Publisher | Year | Focus |
|---|---|---|---|---|
| [*Learning eBPF*](https://www.oreilly.com/library/view/learning-ebpf/9781098135119/) | Liz Rice | O'Reilly | 2023 | What eBPF is, how programs attach, and what they can observe |
| [*BPF Performance Tools*](https://www.brendangregg.com/bpf-performance-tools-book.html) | Brendan Gregg | Addison-Wesley | 2019 | bcc/bpftrace tools for Linux observability and performance |
| [*The Rust Programming Language*](https://doc.rust-lang.org/book/) | Steve Klabnik, Carol Nichols | No Starch Press | 2023 | The Rust language the Aya user-space programs are written in |

## Primary references

- [Aya documentation](https://aya-rs.dev/): the Rust eBPF library this tutorial uses from user space.
- [Linux kernel BPF documentation](https://docs.kernel.org/bpf/): program types, maps, BTF, and the verifier.
- [bpftrace](https://github.com/bpftrace/bpftrace) and [bcc](https://github.com/iovisor/bcc): the reference tracing tools the examples compare against.

Nothing in these chapters or the runnable examples copies text or code from these
works.
