# Mycora — Roadmap

This roadmap is intentionally incremental: each version should be a working,
usable TUI, not a partial skeleton. Scope can shift as the design proves
itself against real usage — treat this as a plan, not a contract.

---

## v0.1 — Core data model & skeleton

Goal: prove the tree model in-memory, no persistence yet.

- [x] `cargo new mycora`, base crate layout (`app.rs`, `ui.rs`, `event.rs`,
      `tree.rs`, `note.rs`)
- [x] Core `Note` struct: id, title, body, parent_id, children ordering
- [x] In-memory tree: create / edit / delete a note
- [x] Minimal ratatui shell: single-pane tree view, keyboard navigation
      (up/down/expand/collapse)
- [x] Basic modal input (normal / insert modes, vim-inspired)

## v0.2 — Local persistence (Markdown source of truth)

Goal: notes survive a restart, stored as plain text.

- [x] Define file format: one Markdown file per note, YAML frontmatter
      (`id`, `parent`, `order`, `tags`, `created`, `updated`)
- [x] Load a vault directory into the in-memory tree on startup
- [x] Write-through on every edit (no explicit "save" step)
- [x] Config file (`~/.config/mycora/config.toml`): vault path (editor
      integration and keybindings aren't implemented features yet, so
      left out of the config schema rather than stubbed unused)
- [x] Handle file-system edge cases: orphaned files, broken parent
      references, duplicate IDs

## v0.3 — Full tree operations

Goal: all CRUD + structural operations, safely.

- [x] Move: reparent a note or subtree (with cycle detection). Exposed in
      the TUI as Tab/Shift+Tab indent/outdent rather than an arbitrary
      note-picker (that needs the search overlay, v0.4)
- [x] Copy: deep-copy a subtree only (new ids, duplicated content). The
      link-only reference variant is deferred to v0.5 — it's really a
      cross-link with tree presence, and depends on the `links` table that
      doesn't exist until then (resolved 2026-07-06, was blocking v0.3)
- [x] Reorder siblings (`K`/`J`)
- [x] Delete with confirmation; soft-delete/trash option before permanent
      removal — moves to `<vault>/.trash/`, never auto-emptied
- [x] Undo/redo stack for all destructive or structural operations within a
      session (in-memory only, not persisted across restarts)

## v0.4 — SQLite index & baseline search

Goal: fast lookups without scanning the filesystem every time.

- [ ] SQLite schema: `notes` (mirrors frontmatter + path), `tree_edges`,
      `links` (many-to-many)
- [ ] Index rebuild command (`mycora reindex`) — index is always disposable
      and regenerable from the Markdown files
- [ ] Incremental reindex on file change (watch vault directory)
- [ ] SQLite FTS5 virtual table for full-text search over title + body
- [ ] Search overlay in the TUI (fuzzy-ish substring search to start)
- [ ] Tag filtering: filter notes by one or more tags with AND/OR boolean
      logic (baseline set-filtering over the `notes`/tags index, no
      relevance ranking yet — that's v0.6's job)

## v0.5 — Cross-links (the "mycelial" layer)

Goal: notes can reference each other outside the tree.

- [ ] Parse `[[wikilink]]` syntax from note bodies
- [ ] Persist links in the `links` table, independent of tree position
- [ ] Backlinks panel: "notes that link here"
- [ ] Link autocompletion while typing `[[`
- [ ] Handle broken links (target renamed/deleted) gracefully
- [ ] Link-count badge on collapsed tree branches: aggregate link count
      across the collapsed subtree (e.g. `▸ Research (12 links)`), computed
      on the fly from the `links` table rather than cached — expected to
      stay well under the 50ms search-latency budget even at thousands of
      notes

## v0.6 — Search engine upgrade (tantivy)

Goal: relevance-ranked search, not just substring matches.

- [ ] Introduce tantivy as the primary full-text index, fed from the same
      Markdown source
- [ ] BM25-ranked results; snippet/highlight generation
- [ ] Faceted filters combined with ranked results: tag (building on
      v0.4's AND/OR tag filter), date range, tree branch
- [ ] Benchmark tantivy vs. FTS5 on a realistic vault size before fully
      committing (keep FTS5 as fallback if tantivy adds too much overhead
      for small vaults)

## v0.7 — UX polish

Goal: make daily use pleasant, not just functional.

- [ ] Configurable keybindings
- [ ] Theming (at minimum: light/dark, respecting terminal colors)
- [ ] Split-pane layout: tree + note body + backlinks, resizable
- [ ] Render note body as formatted markdown in the preview pane, built on
      `pulldown-cmark` (already in the stack for wikilink extraction)
      rather than a dedicated rendering crate — evaluated `ratatui-markdown`
      (2026-07) and passed: too young (2 months old, 12 releases, API still
      moving) and pinned to ratatui ^0.29 vs. our 0.30
- [ ] Command palette (`:` command mode, à la vim/helix)
- [ ] Session state: remember last open note, expanded/collapsed branches
- [ ] 2-line status bar, harmonized with Terapi/jsoned: `Length(2)` band
      split into two `Length(1)` rows, `Color::Indexed(236)` background.
      Row 1: contextual breadcrumb (`vault › branch › note`). Row 2:
      keybinding hints, styled per terapi's hint-parser (bold key tokens,
      dim separators) rather than jsoned's plain concatenated string.
- [ ] No top-level Tabs bar for now — Mycora's single-view-with-panels
      layout (tree + editor + backlinks) matches jsoned's model, not
      terapi's multi-view one. Revisit only if a genuinely separate
      top-level view emerges (e.g. a tree view vs. a link/graph view).

## v0.8 — Import / export & interoperability

Goal: notes are never trapped in Mycora.

- [ ] Import from an existing Obsidian-style vault (wikilinks + frontmatter)
- [ ] Export a subtree to a single flattened Markdown document
- [ ] Optional Postman/Terapi-style templating hooks (stretch — evaluate
      whether this belongs here or in a separate tool)

## v0.9 — Hardening

Goal: stability before a public release.

- [ ] Test coverage on tree operations (especially move/copy/delete edge
      cases) and link integrity
- [ ] Crash-safety: no data loss on unexpected exit (atomic writes)
- [ ] Large-vault performance pass (thousands of notes)
- [ ] Documentation: user guide, keybinding reference, file format spec

## v1.0 — Public release

- [ ] Publish to crates.io
- [ ] `PUBLISH.md` / release checklist (mirroring the Terapi process)
- [ ] Announce, gather feedback, triage into a v1.x backlog

---

## Open design questions

- ~~**Copy semantics**~~ — resolved 2026-07-06: v0.3 implements deep-copy
  only. Link-only reference copy is really a cross-link with tree
  presence, deferred to v0.5 once the `links` table exists (see v0.3).
- ~~**Note identity**~~ — resolved in v0.2: UUID v4, generated at creation
  and persisted in frontmatter. Stable across renames/moves.
- **Multiple vaults**: single vault per instance, or support switching
  between several vaults without restarting?
