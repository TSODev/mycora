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
      `mycora reindex` against real vault files.
- [x] Cross-vault links: a wikilink can resolve to a note in any *mounted*
      vault, not just the current one (see "Multiple vaults" below) — this
      is the intended path for referencing another vault's content, since
      trees themselves stay single-vault (no cross-vault reparenting).
      Required a `links` schema change: a single `vault_id` column can't
      represent an edge whose two ends live in different vaults, so it's
      now `source_vault`/`source`/`target_vault`/`target` (old on-disk
      shape auto-dropped and recreated on open — the index is disposable,
      not worth a real migration for data that regenerates for free).
      `Index::reindex` split into two phases: `write_notes` per vault, then
      `write_links` per vault — link resolution needs every vault's notes
      already written before any of them can be looked up, so it can't be
      interleaved per-vault the way the rest of reindex is. New
      `Index::reindex_mounted(&[(vault_id, tree, vault)])` batches this
      correctly across every vault in the call; the existing single-vault
      `Index::reindex` is now a one-entry convenience wrapper around it, so
      every prior single-vault call site and test kept working unchanged.
      Resolution is deliberately scoped to just the vaults in the batch,
      not "every vault ever indexed" — a vault mounted in a past session
      but not part of this call doesn't get to resolve as a link target,
      so its stale rows (still on disk until something reindexes over
      them) can't silently leak into a session that unmounted it. `App`,
      `mycora reindex`, and `--watch` all now reindex every mounted vault
      as one batch, replacing per-vault loops that couldn't have resolved
      cross-vault links even after the schema/API changes. `backlinks` and
      `link_count_for_subtree` updated for the new column names — both
      already worked cross-vault "for free" once the schema could express
      it. Manually verified: a wikilink in one mounted vault correctly
      resolved to a note in another (via both `mycora reindex` and a fresh
      TUI startup), the target vault's link-count badge picked it up, and
      unmounting the target vault correctly turned the same link "broken"
      rather than resolving it from stale rows
- [x] Backlinks panel: "notes that link here" — `Index::backlinks(vault_id,
      target)` (title-ordered, reads whatever `reindex` last resolved,
      doesn't reindex itself). TUI: `b` in Normal mode reindexes then opens
      `Mode::Backlinks` over the selected note's incoming links, reusing
      the same list/Up/Down/Enter/Esc pattern as the search overlay (`b`
      is a read of the currently selected note, not a query, so no typing
      state needed). Manually verified in tmux: two notes linking to a
      third both showed up, jumping to one moved the tree selection, and
      re-opening backlinks on the newly selected (unlinked) note correctly
      showed an empty list
- [ ] Link autocompletion while typing `[[` — was blocked on a note-body
      editor existing at all; that landed in v0.7 (2026-07-10,
      `ratatui-textarea`-based), so this is unblocked now but still not
      implemented itself
- [x] Handle broken links (target renamed/deleted) gracefully — `reindex`
      now returns a `ReindexReport { note_count, broken_links }` instead of
      a bare count; each unresolved `[[title]]` becomes a `BrokenLink {
      source, title }` rather than being silently dropped. `mycora
      reindex`/`--watch` print one warning per broken link (mirroring how
      `vault.load()`'s own warnings print); `App::new()` folds the same
      warnings into the list already printed before the TUI starts. Since
      link resolution is by title, a rename or delete that leaves a
      `[[title]]` with no match is exactly this case — no special-casing
      needed beyond what "no note has this title" already covers. Manually
      verified via both `mycora reindex` and TUI startup against a vault
      with a genuinely unresolvable link
- [x] Link-count badge on collapsed tree branches: aggregate link count
      across the collapsed subtree (e.g. `▸ Research (12 links)`), computed
      on the fly from the `links` table rather than cached — expected to
      stay well under the 50ms search-latency budget even at thousands of
      notes. `Index::link_count_for_subtree(vault_id, ids)` counts distinct
      `links` rows where source or target is in the subtree (an internal
      link between two notes both inside it still counts once, not twice).
      Shown only when count > 0, to avoid cluttering every collapsed
      leaf-only branch with "(0 links)". Manually verified in tmux: a
      branch with two outgoing wikilinks showed "(2 links)" once collapsed

v0.5 is done except link autocompletion, blocked on the v0.7 body editor
as noted above.

## v0.6 — Search engine upgrade (tantivy)

Goal: relevance-ranked search, not just substring matches.

**Reconsidered 2026-07-10**: the goal above is already met without
tantivy — FTS5 (v0.4) already does BM25 ranking natively (`ORDER BY
rank`, already in `Index::search`) and already ships `snippet()`. So
rather than adding a second full-text engine on spec, this version's
scope shifted to squeezing more out of FTS5 first (snippet generation,
faceted filters) and treating tantivy as something to revisit only if a
concrete gap shows up (typo tolerance, ranking quality at a large vault
size, etc.) — matching the "benchmark before committing" item below,
just resolved before writing the tantivy integration rather than after.

- [ ] Introduce tantivy as the primary full-text index, fed from the same
      Markdown source — **deferred**, not attempted yet; see above
- [x] BM25-ranked results — already true since v0.4 (FTS5's `rank`);
      **snippet/highlight generation** added now: `Index::search` returns
      `SearchHit { note_id, title, snippet }`, `snippet` built via FTS5's
      own `snippet()` function (body column, `…` ellipsis, 16-token
      window), with each matched term wrapped in `\u{1}`/`\u{2}` sentinel
      characters rather than visible markup — keeps the delimiter choice
      out of index.rs's business and lets a renderer decide how to style
      a match. `ui.rs`'s `spans_from_snippet` splits on those sentinels
      into styled ratatui spans (dim context, bold-yellow match); the
      search overlay now renders a 2-line entry per hit (title + snippet)
      instead of title-only. Manually verified in tmux: searching
      "borrow" against a note containing "borrowing" showed the snippet
      with only that word bold-yellow, rest dimmed
- [x] Faceted filters combined with ranked results: tag (building on
      v0.4's AND/OR tag filter), date range, tree branch —
      `Index::search_faceted(vault_id, query, &SearchFacets { tags,
      date_range, branch })`, every facet ANDed onto the FTS5 match and
      onto each other. `tags` reuses `filter_by_tags`'s AND/OR op; `branch`
      takes explicit note ids (typically `Tree::subtree_ids(root)` from
      the caller) rather than a recursive SQL lookup, since the caller
      already has the tree in memory; `date_range` is an inclusive range
      on `updated`. `search(vault_id, query)` is now a thin wrapper
      (`search_faceted` with every facet `None`), so it and every existing
      caller/test kept working unchanged. Backend/API only, matching how
      v0.4's tag filter landed — this roadmap item didn't call for its own
      TUI surface (no keybinding for picking facets exists), unlike
      full-text search itself
- [ ] Benchmark tantivy vs. FTS5 on a realistic vault size before fully
      committing (keep FTS5 as fallback if tantivy adds too much overhead
      for small vaults) — superseded by the 2026-07-10 note above;
      revisit only if a concrete FTS5 gap shows up

v0.6's goal (relevance-ranked search) is met without tantivy; the two
remaining boxes are the same deferred item, not outstanding work.

## v0.7 — UX polish

Goal: make daily use pleasant, not just functional.

- [x] Note-body editor (2026-07-10) — `e` in Normal mode opens the selected
      note's body in a full-pane overlay (`Mode::EditBody`), built on
      `ratatui-textarea` rather than a hand-rolled multi-line editor: it's
      exactly the kind of easy-to-get-wrong functionality (UTF-8 cursor
      movement, line editing) worth an established crate for. Checked
      compatibility first — `tui-textarea` (the best-known one) is stale
      (Oct 2024) and pinned to the pre-split ratatui `^0.29`, incompatible
      with our 0.30; `ratatui-textarea` targets the same `ratatui-core
      ^0.1`/`ratatui-widgets ^0.3` our 0.30.2 already resolves to, so no
      version conflict. `Esc` saves and exits — deliberately no separate
      discard-without-saving path; a whole edit session is one `u`-undoable
      step if you want to back out after the fact, consistent with the
      rest of the app's "no explicit save" philosophy. A no-op edit
      (nothing changed) skips the disk write and the undo entry entirely.
      **Deliberately full-pane, not the split-pane layout below** — that's
      its own separate item, kept open on purpose rather than folded into
      this one, so a real tree+body+backlinks layout can still be designed
      properly later instead of being backed into by the editor. This also
      retroactively unblocks v0.5's "Link autocompletion while typing
      `[[`" (there's now somewhere to type `[[`) — autocomplete itself
      still isn't implemented, just no longer blocked. Manually verified
      in tmux: existing body loaded correctly, multi-line editing (Enter
      for newlines) worked, `Esc` persisted to disk, `u` correctly
      reverted the file, and a no-change edit session left the file's
      `updated` timestamp untouched
- [ ] Configurable keybindings
- [ ] Theming (at minimum: light/dark, respecting terminal colors)
- [x] Split-pane layout: tree + note body + backlinks (2026-07-10) — three
      columns in Normal/Insert/ConfirmDelete modes, fixed proportions
      (40/40/20). **Not yet resizable**: interactive resizing was
      deliberately kept as its own open item rather than folded in here
      (confirmed with the user before implementing), so it's still
      unchecked below on purpose. The body pane is a read-only plain-text
      preview of the selected note (Markdown rendering is the separate
      item right below, also still open) — it doesn't reuse `Mode::EditBody`'s
      full-pane overlay, which stays exactly as it was; pressing `e` still
      takes over the whole screen rather than editing in-place. The
      backlinks pane is similarly read-only and passive: it follows the
      current selection but doesn't reindex first (same as the link-count
      badges), and jumping to one of its entries still goes through the
      existing interactive `b` overlay (`Mode::Backlinks`) rather than
      being merged into the pane itself — a deliberate scope cut, agreed
      with the user before implementing, to keep this pass to "show a
      third pane" rather than "rebuild backlinks navigation." Manually
      verified in tmux: selecting a different note updated both the body
      and backlinks panes live, and all three full-pane overlays (search,
      the backlinks picker, the body editor) still take over the whole
      screen exactly as before rather than showing the split
- [ ] Resizable panes for the split layout above — kept open on purpose,
      see that entry's note
- [ ] Interactive backlinks pane (jump to an entry without the separate
      `b` overlay) — kept open on purpose, see the split-layout entry's note
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
  **Implemented 2026-07-10** (first pass, see v0.5 above): every registry
  entry with `mounted = true` (the default) loads at startup, each into
  its own `Tree`, all sharing the one `Index` (already `vault_id`-scoped).
  No runtime mount/unmount keybinding yet — "mounted" is decided by
  `config.toml` and re-read on each launch, not toggled mid-session; that
  and persisting "which vaults were open last session" both stay v0.7
  territory as originally scoped above. Bigger deliberate scope cut: only
  `config.active_vault()` (named `"default"`, or the first mounted entry)
  is *editable* — every other mounted vault is read-only in the TUI
  (shown stacked below, `── name ──` separator, roots only, always
  collapsed, never `selected`). Full multi-vault editing needs every
  mutating `App` method to first resolve which vault a given `NoteId`
  belongs to; deliberately not attempted in this pass given how many
  methods that touches. Search (`/`) and backlinks (`b`) are similarly
  scoped to the editable vault only — a jump-to-result lands nowhere for a
  read-only vault's note, so they're left out of both rather than jumping
  to a note the tree can't actually select. Link-count badges *do* work
  for read-only vaults (`Index::link_count_for_subtree` just takes an
  explicit `vault_id`), which is what actually proves the shared index
  works across mounted vaults in this pass.
