---
id: 925b8fa7-2fe7-4836-8f37-603a41d24e86
parent: d6833c8b-2dc2-4cfb-970f-c4d7537c60a0
order: 0
tags:
- roadmap
- built
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T20:00:00Z
---

# Built so far

- **v0.1** — core in-memory tree model, minimal ratatui
  shell, modal input
- **v0.2** — Markdown + YAML frontmatter persistence, config file,
  self-healing on load (see [[Markdown as source of truth]])
- **v0.3** — full [[Tree operations]]: move/reparent with cycle
  detection, deep-copy, reorder, delete-to-trash, [[Undo and redo]]
- **v0.4** — the SQLite index: full-text search (FTS5), tag filtering,
  manual and watched reindex (see [[Search and indexing]])
- **v0.5** — the "mycelial" layer: wikilink parsing, backlinks
  panel, broken-link handling, cross-vault resolution, link-count badges
  (see [[Cross-links and backlinks]])
- **v0.6** — search quality: BM25 snippets/highlighting, faceted filters
  (tag/date/branch); tantivy deliberately deferred (see
  [[Disposable SQLite index]])
- **v0.7** — UX polish: full-pane body editor, theming, split-pane
  [[Layout]] with resizing, interactive backlinks focus, Markdown
  rendering in the body preview, the [[Command palette]], [[Session persistence]] — everything except configurable keybindings (see
  [[Deferred: configurable keybindings]])
- **Since v0.7** — a full `mycora vault ...` CLI for the registry:
  [[Managing vaults from the CLI]] (`add`, `init`, `rename`, `promote`,
  `mount`, `unmount`, `remove`, `list`)
