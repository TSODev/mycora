---
id: 599f041a-3fa6-4fdd-b145-8fb8f7e8a8ef
parent: null
order: 0
tags:
- overview
- welcome
created: 2026-07-14T09:00:00Z
updated: 2026-07-14T09:00:00Z
---

# Jsoned

A keyboard-driven TUI for viewing and editing JSON — "`jless`, but you
can actually edit things."

Structural editing, undo/redo, search, a structural lint pass, a
plugin system (with `jq` bundled), and format conversion between JSON,
YAML, TOML, CSV, and JSONL, all from one single-binary terminal tool —
usable standalone, or as an external editor plugged into other TSODev
tools (`TERAPI_JSON_EDITOR=jsoned`).

This vault documents Jsoned as a Mycora showcase, the same way Mycora
documents itself in its own showcase-vault.

Start here:

- [[Philosophy]] — the gap between JSON viewers and full-blown editors
- [[Architecture]] — the tree model, the patch-based performance
  pattern, the module map
- [[Features]] — what's actually built
- [[Performance]] — what the patch machinery actually buys, measured
- [[Design decisions]] — specific choices, and the reasoning behind
  them
- [[Roadmap]] — what shipped, what's still open
