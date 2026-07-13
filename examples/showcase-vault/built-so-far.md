---
id: 925b8fa7-2fe7-4836-8f37-603a41d24e86
parent: d6833c8b-2dc2-4cfb-970f-c4d7537c60a0
order: 0
tags:
- roadmap
- built
created: 2026-07-10T09:00:00Z
updated: 2026-07-13T18:00:00Z
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
  `mount`, `unmount`, `remove`, `archive`, `unarchive`, `list`);
  [[Read-only secondary vaults]] became fully navigable (not just
  visible), with every mutation guarded against acting on the wrong
  vault (see [[Guard every mutation against the wrong vault]]); an
  *unmounted* vault stopped being invisible too, showing up as its own
  unexpandable placeholder row with a "how to mount it" message (see
  [[Unmounted vaults are visible too]]); an unmounted vault can now be
  compressed down to a single archive file to reclaim disk space, and
  restored back, with its own distinct tree row too (see
  [[Compressing a vault trades files for one archive, deliberately]]);
  `:config unmount show/hide` and `:config archive show/hide` declutter
  the tree once a registry has enough of either
- **v0.8** — notes are never trapped in Mycora: [[Exporting a subtree]]
  flattens a note and its descendants to Markdown or PDF (see
  [[PDF export renders through a pure-Rust crate]]);
  [[Importing an Obsidian vault]] converts an existing Obsidian vault
  into a new one, mapping its folder structure onto Mycora's tree (see
  [[Folder structure becomes tree structure]]); only the optional
  stretch-goal templating hooks are left unstarted
- **Since v0.8** — `:tag add <tag>` / `:tag del <tag>` manage the
  selected note's tags directly, shown as `#tag` badges along the
  bottom of the body preview (see [[Command palette]] and [[Layout]])
- **v0.9** — done, stability before a public release:
  [[Every write to disk is atomic]] closes the crash-safety gap in
  `config.toml`/`session.toml`; an audit of `tree.rs`/`link.rs`'s test
  coverage added 19 tests for untested move/copy/delete edge cases and
  caught a real self-healing gap along the way — a self-parented note
  used to vanish from navigation instead of being healed like any other
  malformed parent (see [[Markdown as source of truth]]); a large-vault
  benchmark pass found and fixed a quadratic `mycora reindex` (see
  [[Reindex was quadratic, one missing index fixed it]]); a documentation
  audit checked USAGE.md against the actual code rather than assuming it
  was current (see [[Roadmap]]), and found (and fixed) several places it
  had quietly gone stale
- **Since v0.9** — every line break typed in the body editor now renders
  as its own line in the preview, even without a blank line between them
  (see [[Layout]]'s body-preview note); the interface itself went
  multilingual — English, French, Spanish, German, switchable live with
  `:lang <en|fr|es|de>` and persisted to `config.toml` (see
  [[The interface speaks four languages]]); every mounted vault gets a
  centered, background-colored name header in the tree pane (see
  [[Layout]]); the body editor now offers wikilink autocompletion as you
  type (see [[Cross-links and backlinks]]) — the last of the two
  long-deferred headline items from early on, open since v0.5; `f`
  follows a note's outgoing wikilinks, the backlinks pane's exact mirror
  image (see [[Cross-links and backlinks]]); and the status bar got two
  fixes at once — Normal mode's hint row, grown to 233 characters over
  several versions, is now a short curated set plus a `?` full-pane
  reference for everything else, and the breadcrumb row gained a
  centered "last modified" timestamp, shown only when there's room
  (see [[Status bar]])
