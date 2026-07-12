---
id: 63bdb3f5-f8a3-470a-b379-5b864d3a6dff
parent: d6833c8b-2dc2-4cfb-970f-c4d7537c60a0
order: 1
tags:
- roadmap
- planned
created: 2026-07-10T09:00:00Z
updated: 2026-07-12T11:00:00Z
---

# What's still ahead

- **Link autocompletion** while typing a wikilink in the
  body editor — unblocked now that [[Full-pane body editor, save on exit]]
  exists, but not yet implemented
- **v0.8 — Import/export**: [[Exporting a subtree]] (Markdown or PDF)
  and [[Importing an Obsidian vault]] are both done; still ahead:
  optional Postman/Terapi-style templating hooks (stretch goal, may not
  belong in Mycora itself)
- **v0.9 — Hardening (in progress)**: [[Every write to disk is atomic]],
  the tree/link test-coverage audit (see
  [[Markdown as source of truth]]), and the large-vault performance pass
  (see [[Reindex was quadratic, one missing index fixed it]]) are all
  done; still ahead: a full documentation pass
- **v1.0 — Public release**: crates.io publish, release checklist,
  gather feedback

[[Deferred: configurable keybindings]] stays explicitly out of scope
until real friction shows up in practice, rather than being scheduled
into any of the above.
