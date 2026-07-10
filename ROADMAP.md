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

- [x] SQLite schema: `notes` (mirrors frontmatter + path), `tree_edges`,
      `links` (many-to-many) — each keyed with a `vault_id` from the start
      (see "Multiple vaults" below) so multi-vault support doesn't require
      a schema migration later. `links` exists but stays empty until v0.5
      parses wikilinks. Index lives at `~/.local/share/mycora/index.sqlite3`
      (XDG data dir, not `~/.config`, since it's generated/disposable) via
      `rusqlite` (`bundled` feature, no system libsqlite3 dependency)
- [x] Index rebuild command (`mycora reindex`) — index is always disposable
      and regenerable from the Markdown files; rebuilds only the active
      vault's rows (`config.active_vault()`), scoped by `vault_id`
- [x] Incremental reindex on file change (watch vault directory) —
      `mycora reindex --watch`, via the `notify` crate, non-recursive
      (`Vault::load` doesn't recurse either, so this matches). "Incremental"
      means *event-triggered*, not a per-file diff: each debounced batch of
      filesystem events (300ms coalescing window, since one atomic save is
      often a write + rename-into-place) still does a full `vault.load()` +
      `index.reindex()` for the active vault — consistent with the index
      being disposable and "cheaper to regenerate wholesale than to diff"
      (see `Index::reindex`'s doc comment). Manually verified: adding and
      removing a note file while `--watch` was running correctly bumped the
      indexed count up then back down
- [x] SQLite FTS5 virtual table for full-text search over title + body —
      `notes_fts` (title, body, tags), rebuilt alongside `notes`/`tree_edges`
      in `reindex`. `Index::search()` turns free-text input into an ANDed,
      per-term prefix match (`"term"*`) rather than exposing raw FTS5 query
      syntax to callers; ranked by FTS5's built-in `rank`
- [x] Search overlay in the TUI (fuzzy-ish substring search to start) — `/`
      in Normal mode enters `Mode::Search`, reindexing first so results
      reflect the live in-memory tree rather than a stale on-disk index.
      Results update on every keystroke; Up/Down cycles them; Enter expands
      the hit's ancestors and selects it in the tree; Esc cancels without
      touching the current selection
- [x] Tag filtering: filter notes by one or more tags with AND/OR boolean
      logic (baseline set-filtering over the `notes`/tags index, no
      relevance ranking yet — that's v0.6's job) — new `tags` table
      (`vault_id`, `note_id`, `tag`), populated in `reindex`.
      `Index::filter_by_tags(vault_id, tags, TagFilterOp::{All,Any})`.
      Backend/index only, matching this bullet's scope as written (unlike
      full-text search, this item never called for its own TUI overlay);
      a tag-browsing UI is left for whenever v0.7's UX polish or a later
      pass picks it up

v0.4 is now feature-complete against this list.

## v0.5 — Cross-links (the "mycelial" layer)

Goal: notes can reference each other outside the tree.

- [x] Parse `[[wikilink]]` syntax from note bodies — `link::extract_wikilink_titles`,
      a small hand-rolled bracket scanner (no `regex` dependency added for
      this). Stops cleanly at an unclosed `[[` rather than erroring
- [x] Persist links in the `links` table, independent of tree position —
      resolved and (re)written by `reindex`, scoped by `vault_id` like
      every other table. Titles aren't required to be unique, so a
      wikilink whose title matches more than one note fans out to a link
      per match (resolved 2026-07-10, in the "Multiple vaults" spirit of
      not silently guessing); a title matching no note is simply skipped
      (that's what "broken link" means here) rather than erroring, and a
      note linking to its own title is skipped too. Manually verified via
      `mycora reindex` against real vault files. Backlinks *querying* isn't
      exposed yet (no `Index` method beyond what's needed for `reindex`
      itself) — that's the next item
- [ ] Cross-vault links: a wikilink can resolve to a note in any *mounted*
      vault, not just the current one (see "Multiple vaults" below) — this
      is the intended path for referencing another vault's content, since
      trees themselves stay single-vault (no cross-vault reparenting)
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
- ~~**Multiple vaults**~~ — resolved 2026-07-10: a registry/mount split,
  not a merge. `config.toml` gains a list of named vaults (registry:
  `name` + `path` per entry, replacing the single `vault_path`); `App`
  holds a `VaultId → (Tree, Vault)` map for whichever of those are
  currently *mounted* (loaded), and mounting/unmounting is a runtime
  action with nothing to persist beyond "which vaults were open last
  session" (v0.7 session state). Each mounted vault keeps its own
  independent `Tree` with its own `roots` — deliberately **not** merged
  into one shared tree/root, since that would require either a synthetic
  super-root or allowing `move_note` to reparent across vaults, and the
  latter breaks `vault.rs`'s "one `Vault` = one on-disk directory"
  invariant (a cross-vault move would mean moving a file between two
  independent directory trees, which no current `Vault` method does).
  `tree.rs`/`vault.rs` stay untouched by this — mount/unmount lives at
  the `App` layer only. Cross-vault references are deferred to v0.5's
  `[[wikilink]]` links rather than tree reparenting; this is also why the
  v0.4 SQLite schema needs `vault_id` on `notes`/`tree_edges`/`links` from
  its first version.
