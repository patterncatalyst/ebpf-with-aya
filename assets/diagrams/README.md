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

Shipped: `lab-topology` (host + target VM + peer VM + containers),
embedded in Chapter 2. Planned next: the kernel→map→user-space→OTLP
data path (Ch 3) and the load/verify/attach lifecycle (Ch 5). Concepts
not yet drawn are shown as ASCII diagrams inline in the chapters.

The committed `.svg` is what renders on the site; the paired
`.excalidraw` is the editable source — open it at excalidraw.com, edit,
and re-export the SVG (File → Export image → SVG) to update.

When exporting SVGs, use `viewBox="0 0 W H"` without fixed `width`/
`height` so the CSS scales them responsively. Hard-refresh the browser
after updating an SVG — they cache aggressively.
