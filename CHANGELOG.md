# Changelog

All notable changes to this project will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Added
- **Resizable split-pane layout (v0.7)** — `[`/`]` shrink/grow the tree
  pane, `{`/`}` shrink/grow the backlinks pane, always active in Normal
  mode (no dedicated resize mode). The body pane is never resized
  directly — it's the middle column, so it just absorbs whatever width
  the other two give up or take. Floor of 10% per pane, 5% per keypress.
  In-memory only, not persisted in `session.toml`: pane widths are a
  display preference, not per-vault navigation state, so they reset to
  the 40/40/20 default each launch.

### Changed
- **Interactive backlinks pane replaces the `b` overlay (v0.7)** — `b` no
  longer opens a separate full-screen overlay; it shifts keyboard focus
  onto the backlinks pane already visible in the split layout instead.
  `j`/`k`/`Up`/`Down` move within it, `Enter` jumps (same ancestor-reveal
  behavior as before), `Esc` or `b` again returns focus to the tree. The
  focused pane gets a cyan border and reversed-highlight, matching the
  tree's own selection styling. Also drops the reindex-on-open the old
  overlay did — the pane now reads `App::live_backlinks()` exactly like
  the passive pane already did, so `b` no longer forces a fresh reindex.

### Added
- **Session state (v0.7)** — Mycora now remembers the last selected note
  and which branches were expanded/collapsed, per vault, across restarts.
  New `src/session.rs` reads/writes `~/.local/share/mycora/session.toml`.
  Saved once at shutdown (covers both `q`/`q` and `Ctrl+C` uniformly, no
  per-keystroke writes) and restored in `App::new` — ids that no longer
  exist are dropped, and the restored selection's ancestors are always
  expanded so it's actually visible.

### Changed
- **2-line status bar (v0.7)**, harmonized with Terapi/jsoned — a
  `Length(2)` band split into two `Length(1)` rows, `Color::Indexed(236)`
  background on both. Row 1 is a contextual breadcrumb (`vault › branch ›
  note`, via new `App::vault_name()`/`App::breadcrumb_titles()`). Row 2 is
  the mode label plus keybinding hints, now tokenized on a `"key: label"`
  convention into bold key / dim colon / muted label spans instead of a
  plain concatenated string. The delete-confirmation prompt,
  quit-confirmation notice, and last-error message still take over row 2
  as before, just leaving row 1's breadcrumb visible above them now
  instead of replacing the whole bar.

### Added
- **Markdown rendering in the body preview pane (v0.7)** — a new
  `src/markdown.rs` module walks `pulldown-cmark`'s event stream and
  builds styled ratatui lines directly, rather than pulling in a
  dedicated ratatui-markdown crate (re-evaluated `ratatui-markdown` and
  it's still pinned to ratatui `^0.29`, incompatible with our 0.30).
  Covers headings (color-coded by level), bold/italic, inline/block code,
  bulleted/numbered lists (with nesting), blockquotes, and horizontal
  rules. Not interactive — links and `[[wikilinks]]` render as plain
  text.
- **Split-pane layout (v0.7)** — Normal/Insert/ConfirmDelete modes now
  show three columns (fixed 40/40/20 proportions): the tree, a read-only
  plain-text preview of the selected note's body, and a read-only
  backlinks list that both follow the current selection live. Interactive
  resizing, an interactive backlinks pane (jump without the separate `b`
  overlay), and Markdown rendering in the body pane are all deliberately
  left for later — this pass is just the three-pane skeleton. Search, the
  backlinks picker, and the body editor still take over the whole screen
  as full-pane overlays, unchanged.
- **Note-body editor (v0.7, start)** — `e` in Normal mode opens the
  selected note's body in a full-pane overlay (`Mode::EditBody`), built on
  the `ratatui-textarea` crate. `Esc` saves and returns to Normal
  (persist-on-exit, not per-keystroke); a whole edit session is one
  `u`-undoable step. A no-op edit (nothing changed) skips both the disk
  write and the undo entry. Deliberately full-pane rather than the
  separate split-pane (tree + body + backlinks) layout item, which stays
  open for later. Also retroactively unblocks v0.5's link autocompletion
  (there's now somewhere to type `[[`), though autocomplete itself isn't
  implemented yet.
- **Faceted search filters (v0.6, closes the version except deferred
  tantivy)** — `Index::search_faceted(vault_id, query, &SearchFacets {
  tags, date_range, branch })` ANDs optional tag (AND/OR, reusing v0.4's
  `filter_by_tags` op), update-date-range, and tree-branch (explicit note
  ids, typically `Tree::subtree_ids(root)`) facets onto an FTS5 text
  match. `search(vault_id, query)` is now a thin wrapper around it with
  every facet `None`, so existing callers/tests are unaffected.
  Backend/API only, no TUI surface yet — matches how v0.4's tag filter
  landed.
- **Search result snippets (v0.6, start)** — `Index::search` now returns
  `SearchHit { note_id, title, snippet }` instead of the plain
  note_id+title pair; `snippet` comes from FTS5's own `snippet()`
  function, with each matched term wrapped in sentinel characters (never
  shown directly) that `ui.rs` splits on to style the match distinctly.
  The search overlay now shows a 2-line entry per result (title + snippet,
  matched term bold-yellow) instead of title-only.

### Changed
- **v0.6 no longer plans to adopt tantivy on spec** — FTS5 (v0.4) already
  does BM25 ranking and ships its own `snippet()`; the roadmap's
  "benchmark before committing" was resolved by not writing the
  integration in the first place. Revisit only if a concrete FTS5 gap
  shows up. See ROADMAP.md's v0.6 section for the full reasoning.

### Added
- **Cross-vault `[[wikilink]]` resolution (v0.5, closes the version except
  autocompletion)** — a wikilink in one mounted vault can now resolve to a
  note in any other mounted vault, not just its own. Required reshaping
  the `links` table (`source_vault`/`source`/`target_vault`/`target`
  instead of one `vault_id` column that couldn't represent a cross-vault
  edge; the old shape is auto-dropped and recreated on open, no real
  migration since the index is disposable) and splitting `Index::reindex`
  into two phases (`write_notes` per vault, then `write_links` per vault,
  since link resolution needs every vault's notes already written first).
  New `Index::reindex_mounted(&[(vault_id, tree, vault)])` batches this
  correctly across a set of vaults; the existing single-vault
  `Index::reindex` is now a one-entry convenience wrapper around it.
  Resolution is scoped to just the vaults in a given batch, not every
  vault ever indexed, so an unmounted vault's stale rows can't silently
  resolve as a link target. `App`, `mycora reindex`, and `--watch` all
  reindex every mounted vault as one batch now. `backlinks` and
  `link_count_for_subtree` work cross-vault too, now that the schema can
  express it.
- **Multi-vault mounting, read-only for now** — every registry entry with
  `mounted = true` (the default) now loads at startup, each into its own
  `Tree`, sharing the one `vault_id`-scoped `Index`. Only the editable
  vault (`config.active_vault()`) is navigable/selectable; every other
  mounted vault shows up stacked below it with a `── name ──` separator,
  roots only, always collapsed, read-only — `j`/`k` never selects into
  it. Its notes are still indexed, so link-count badges work on it, but
  search (`/`) and backlinks (`b`) stay scoped to the editable vault only
  (jumping to a result in a read-only vault has nowhere to land). `mycora
  reindex`/`--watch` now cover every mounted vault, not just the active
  one. Full multi-vault editing (every mutating `App` method resolving
  which vault a note belongs to) is deferred to a later pass — see
  ROADMAP.md's "Multiple vaults" entry for the full scope writeup.
  `VaultEntry` gains a `mounted: bool` field (`config.toml`'s `[[vaults]]`
  entries), defaulting to `true`.
- **Link-count badge on collapsed branches (v0.5)** — a collapsed note
  with children shows an aggregate link count, e.g. `▸ Research (2
  links)`, when that count is greater than zero. `Index::link_count_for_subtree`
  counts distinct `links` rows touching any note in the subtree (source
  or target), counting an internal link between two notes both inside
  it once, not twice. Computed fresh on every render rather than
  cached, per ROADMAP.md's v0.5 entry.
- **Broken link reporting (v0.5)** — `Index::reindex` now returns a
  `ReindexReport { note_count, broken_links }` instead of a bare count;
  a `[[title]]` that resolves to no note becomes a `BrokenLink` entry
  instead of being silently dropped. `mycora reindex`/`--watch` print a
  warning per broken link; `App::new()` surfaces the same warnings
  before the TUI starts, alongside the existing vault-load warnings.

### Changed
- **`Index::reindex`'s return type** — was `Result<usize>`, now
  `Result<ReindexReport>` (`.note_count` replaces the old bare count).
  Source-breaking for anything calling it directly.

### Added
- **Backlinks panel (v0.5)** — `b` in Normal mode reindexes, then opens a
  panel listing notes that link to the selected one (`Index::backlinks`),
  reusing the search overlay's Up/Down/Enter/Esc pattern: Enter jumps to
  the selected backlink (expanding its ancestors), Esc cancels leaving
  the current selection untouched.
- **`[[wikilink]]` parsing and link persistence (start of v0.5)** — a new
  `link` module extracts `[[title]]` occurrences from note bodies (a small
  hand-rolled scanner, no new dependency), and `reindex` resolves each
  title against the vault's notes and writes the resolved pairs into the
  index's `links` table. Titles aren't required to be unique: a wikilink
  matching several notes links to all of them rather than silently
  guessing one; a title matching no note is skipped (a "broken" link);
  self-links are skipped. No querying API yet (backlinks panel, etc.) —
  this lands the extraction + persistence half of v0.5's cross-links work
  first.
- **Tag filtering (v0.4, closes the version)** — a new `tags` table
  (`vault_id`, `note_id`, `tag`), populated by `reindex`.
  `Index::filter_by_tags(vault_id, tags, TagFilterOp::{All,Any})` does
  baseline AND/OR set-filtering, no relevance ranking (that's v0.6's
  tantivy work). Backend/index only — this roadmap item never called for
  a TUI overlay the way full-text search did, so there's no keybinding
  for it yet. `Index::SearchHit` is renamed `IndexedNote` since it's now
  shared between full-text search and tag-filter results.
- **`mycora reindex --watch` (v0.4)** — stays running and reindexes the
  active vault automatically whenever a file in it changes, via the
  `notify` crate (non-recursive, matching `Vault::load`). Debounces
  bursts of filesystem events (300ms) into a single reindex, since one
  atomic save is often a write + rename-into-place. Each trigger is a
  full rebuild of the vault's index rows, not a per-file diff — same
  "disposable, cheap to regenerate wholesale" index philosophy as
  `mycora reindex`, just triggered by file events instead of manually.
- **Search overlay in the TUI (v0.4)** — `/` in Normal mode opens a search
  prompt; results from `Index::search` update live as you type, Up/Down
  cycles them, Enter expands the hit's ancestors and selects it in the
  tree, Esc cancels without touching the current selection. Reindexes
  from the live in-memory tree on entry, so results are never stale
  relative to unsaved-to-index edits made this session.
- **FTS5 full-text search (v0.4)** — `notes_fts` virtual table over title +
  body (+ tags), populated by `reindex` alongside `notes`/`tree_edges`.
  `Index::search(vault_id, query)` turns free-text input into an ANDed,
  per-term prefix match rather than exposing raw FTS5 syntax, ranked
  best-first.
- **SQLite index & `mycora reindex` (start of v0.4)** — a disposable index
  (`notes`, `tree_edges`, `links`, each keyed by `vault_id`) at
  `~/.local/share/mycora/index.sqlite3`, rebuilt from the active vault's
  Markdown files by the new `mycora reindex` subcommand. Nothing reads
  from the index yet (search/FTS5, the watch-driven incremental reindex,
  and tag filtering are the rest of v0.4, still to come) — this lands the
  schema and the rebuild path first. Adds `rusqlite` (`bundled` feature)
  as a dependency.
- **Vault registry in config** — `config.toml` can now declare multiple
  named vaults via `[[vaults]]` (`name` + `path` each) instead of a single
  `vault_path`. Only one vault is opened at startup for now: the entry
  named `"default"`, or the first entry if none is named that — actually
  mounting more than one vault at once (independent trees, switchable at
  runtime) is tracked separately in ROADMAP.md and not implemented yet.
  The older single `vault_path` key still works as a fallback when
  `[[vaults]]` is absent, so existing config files keep working unchanged.
- **`--help` and `--version`** via `clap` — matches Terapi/jsoned's CLI
  conventions. No other flags/arguments yet.

### Fixed
- **A panic while the TUI was running left the terminal broken** — raw mode
  and the alternate screen were only ever torn down on the normal exit
  path, so a panic anywhere skipped that cleanup, leaving garbled input and
  an invisible cursor until the user ran `reset`/`stty sane`. A panic hook
  installed at the top of `main()` now restores the terminal before
  letting the default panic report print. Matches Terapi/jsoned.

### Changed
- **`q` now requires two presses to quit** — a single stray `q` used to
  close the app immediately with no way back. First press arms a
  confirmation (status bar shows "Press q again to quit"); a second `q`
  right after actually quits, any other key cancels it. Matches Terapi's
  existing q/q convention.
- **`Ctrl+C` quits immediately** — crossterm raw mode disables SIGINT
  generation so it previously did nothing. Now handled unconditionally
  before mode dispatch, bypassing the q/q confirm dance and the delete
  confirmation prompt alike — matches Terapi/jsoned.

### Added
- **Full tree operations (v0.3)** — move (Tab/Shift+Tab indent/outdent, with
  cycle detection), deep-copy a subtree (`y`), reorder siblings (`K`/`J`),
  and delete with a y/n confirmation prompt. Delete no longer promotes
  children to the grandparent — it now removes the whole confirmed
  subtree together, moved to `<vault>/.trash/` rather than erased (trash
  is never auto-emptied). Every one of these, plus rename, is undoable
  (`u`) and redoable (`Ctrl+R`) for the rest of the session — undo/redo
  always re-derives its inverse from the *current* live tree state, so a
  chain of undo/redo stays correct even across intervening edits.
- **Markdown persistence (v0.2)** — notes now survive a restart as one
  Markdown file per note in a flat vault directory: YAML frontmatter
  (`id`, `parent`, `order`, `tags`, `created`, `updated`) plus a leading
  `# H1` for the title. Every create/rename/delete writes through
  immediately (atomic: temp file + rename), no explicit save step.
  `NoteId` is now a UUID v4 generated at creation — replaces v0.1's
  in-memory `usize` counter, resolving the note-identity open design
  question. Malformed files, duplicate ids, and notes with an
  unresolvable parent are self-healed and reported as warnings on
  load rather than causing a crash or silent data loss. Vault path is
  configurable via `~/.config/mycora/config.toml`.
- **Test-vault generator** (`examples/generate-test-vault.rs`) — builds
  a synthetic vault (category → sub-category → leaf notes, random tags,
  `[[wikilink]]` cross-references) for TUI load-testing. Reuses
  `mycora::vault::Vault` directly so its output is guaranteed to match
  the app's real on-disk format. Split the crate into `lib.rs` + `main.rs`
  to make this possible.

---

## [0.1.0] — 2026-07-06

### Added
- **In-memory tree skeleton (v0.1)** — initial `Note`/`Tree` core model
  (create, rename, delete with child promotion to the deleted note's
  parent), a minimal ratatui TUI shell with vim-inspired normal/insert
  modal input, and single-pane tree navigation with expand/collapse.
  No persistence yet — notes exist only for the process lifetime.
- Published to [crates.io](https://crates.io/crates/mycora), dual-licensed
  MIT OR Apache-2.0.
