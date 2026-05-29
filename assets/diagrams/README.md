# Diagrams

Each diagram is a pair:

- `<name>.svg` — the rendered version embedded in pages
- `<name>.excalidraw` — the editable source (open at excalidraw.com)

Embed one in a chapter with the include:

```liquid
{% include excalidraw.html
   file="05-ebpf-load-attach"
   alt="How an eBPF program is loaded, verified, and attached"
   caption="Figure 5.1 — Load, verify, attach" %}
```

Diagrams are added in later iterations (see the iteration roadmap).
Planned early ones: the load/verify/attach lifecycle (Ch 5), the
kernel→map→user-space→OTLP data path (Ch 3), and the lab topology
(host + target + peer, Ch 2). Until then those concepts are shown as
ASCII diagrams inline in the chapters.

When exporting SVGs, use `viewBox="0 0 W H"` without fixed `width`/
`height` so the CSS scales them responsively. Hard-refresh the browser
after updating an SVG — they cache aggressively.
