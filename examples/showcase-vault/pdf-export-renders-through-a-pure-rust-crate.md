---
id: b3fe8aee-d701-4ac0-ba2b-6e607c22897f
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 10
tags:
- design-decision
- export
- v0.8
created: 2026-07-11T11:30:00Z
updated: 2026-07-11T11:30:00Z
---

# PDF export renders through a pure-Rust crate

[[Exporting a subtree]] gained a second output format: a `.pdf` path
renders a paginated PDF instead of Markdown. Two forks, both resolved
before writing any code.

**Rendering approach.** The alternative to a Rust dependency was
shelling out to an already-installed tool (`pandoc`, `wkhtmltopdf`) —
zero new crate weight, but `pandoc` alone doesn't actually produce a
PDF without a LaTeX toolchain behind it, and `wkhtmltopdf` is a
largely-unmaintained binary that may not be on the user's machine at
all. Neither is reliable enough to be the *only* way to get a PDF out
of Mycora. Landed on
[`markdown2pdf`](https://crates.io/crates/markdown2pdf) instead — a
self-contained, actively-maintained crate that takes Markdown straight
in and a laid-out PDF straight out (headings, bold/italic, code, lists,
links), so it behaves identically on every machine with no external
install step. Checked the actual crate source before committing to it,
not just its docs — confirmed it's pinned to a current `printpdf 0.9`,
unlike `genpdf` and its forks, which are all years-stale and pinned to
`printpdf 0.3.4`. Its optional `fetch`/`svg` cargo features (network
image fetching, SVG rasterization) are both left off — Mycora doesn't
need either.

**Command surface.** Rather than a new `:print`/`mycora print` command
— floated as an idea, considered, and set aside — the existing
`:export`/`mycora export` just infers the format from the output
path's extension (`.pdf` → PDF, anything else → Markdown). One command
to document and remember instead of two that do almost the same thing.

`export::write_output` is the one place both the TUI and CLI export
paths call to actually write the file, so the extension check lives in
exactly one place rather than being duplicated at each call site.
