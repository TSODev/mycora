---
id: d74fbdfa-cfbf-4827-9a42-d05bae30e309
parent: 599f041a-3fa6-4fdd-b145-8fb8f7e8a8ef
order: 1
tags:
- architecture
created: 2026-07-14T09:04:00Z
updated: 2026-07-14T09:04:00Z
---

# Architecture

- [[JNode, kept deliberately separate from serde_json::Value]] — the
  tree that carries cursor, fold state, and undo history
- [[The patch pattern: refresh_at, not a full rebuild]] — the core
  performance idea
- [[Undo and redo still clone the whole tree]] — the one place the
  patch pattern doesn't reach yet
- [[Diff gets its own read-only app]]
- [[The plugin system is narrow and compiled-in, on purpose]]
- [[Four entry points, one binary]] — TUI, headless diff, headless
  convert, diff TUI

See [[Performance]] for what the patch pattern above actually buys,
measured.
