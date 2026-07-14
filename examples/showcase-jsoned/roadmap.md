---
id: 8289c569-d0bc-4330-a720-3d39477f57da
parent: 599f041a-3fa6-4fdd-b145-8fb8f7e8a8ef
order: 5
tags:
- roadmap
created: 2026-07-14T09:35:00Z
updated: 2026-07-14T09:35:00Z
---

# Roadmap

Shipped, checked off in ROADMAP.md: v0.1 (viewer), v0.2 (scalar
editing), v0.3 (node actions), v0.4 (structural editing), v0.5 (search
+ navigation), v0.6 (save-as, mostly — opening a second format from an
already-open document is still unchecked), v0.7 (structural lint), and
the plugin system/`jq` (shipped ahead of schedule, superseding the
original v0.9 "jq filter" item). Several v0.9 items also landed ahead
of schedule: structural diff, redact on export, and the large-file
performance work in [[Performance]].

Still open: **v0.8, JSON Schema validation** — a `--schema` flag,
`$schema` auto-detection, validation on load and on every edit,
red-highlighted failing nodes, an error count in the status bar, via
the `jsonschema` crate (Draft 4 through 2020-12) — fully unchecked, not
started. The remaining v0.9 items: more plugins (code generation, web
import-and-prune — noted the `Plugin` trait will likely need to grow
beyond JNode-in/JNode-out for these), and multi-tab (open several files
at once, `Tab` to switch between them).

Latest released version per `CHANGELOG.md`: **0.5.1** (keymap
reference, the Ctrl+C fix in [[Ctrl+C is checked first in every mode, not just Normal]], the panic hook in [[A panic hook restores the terminal before the report prints]]); `[Unreleased]` is currently
empty.
