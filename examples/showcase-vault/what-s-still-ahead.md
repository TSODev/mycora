---
id: 63bdb3f5-f8a3-470a-b379-5b864d3a6dff
parent: d6833c8b-2dc2-4cfb-970f-c4d7537c60a0
order: 1
tags:
- roadmap
- planned
created: 2026-07-10T09:00:00Z
updated: 2026-07-13T16:00:00Z
---

# What's still ahead

- **v0.8 — Import/export**: [[Exporting a subtree]] (Markdown or PDF)
  and [[Importing an Obsidian vault]] are both done; still ahead:
  optional Postman/Terapi-style templating hooks (stretch goal, may not
  belong in Mycora itself)
- **v0.9 — Hardening**: done. [[Every write to disk is atomic]], the
  tree/link test-coverage audit (see [[Markdown as source of truth]]),
  the large-vault performance pass (see
  [[Reindex was quadratic, one missing index fixed it]]), and a
  documentation audit against the actual code (USAGE.md had drifted in
  several places — see [[Roadmap]] v0.9's last entry) are all done
- **Since v0.9**: [[The interface speaks four languages]]; the
  body-preview newline fix (see [[Layout]]); vault-name headers in the
  tree pane; and **link autocompletion** — the last of the two
  long-deferred headline items, open since v0.5, finally closed out
  (see [[Cross-links and backlinks]])
- **v1.0 — Public release**: crates.io publish, release checklist,
  gather feedback

[[Deferred: configurable keybindings]] stays explicitly out of scope
until real friction shows up in practice, rather than being scheduled
into any of the above.
