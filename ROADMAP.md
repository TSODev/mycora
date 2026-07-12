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
- [ ] ~~Arbitrary configurable keybindings~~ — **deferred 2026-07-10**, no
      target version. The current bindings are already vim-inspired and
      coherent (`j/k/h/l`, `/` to search, `u` to undo), matching exactly
      the audience a terminal note-taking tool draws — full remapping adds
      a real, permanent cost (a remap config schema, conflict validation,
      docs to maintain, every future feature having to register with it)
      for a need that's speculative until someone actually hits it.
      Revisit only if real friction shows up. If it does, prefer a small
      set of **named presets** (`vim`, maybe `emacs` if there's ever
      demand) over letting every key be individually rebound — covers the
      realistic case (someone's muscle memory doesn't match the default)
      without the maintenance burden of arbitrary per-key remapping.
- [x] Theming, light/dark baseline (2026-07-10) — every explicit color in
      the app is a named ANSI color (`Color::Blue`, `Cyan`, `Yellow`,
      `Red`, `Green`, `Gray`, ...), not RGB or 256-color indices, with one
      deliberate exception: the status bar's `Color::Indexed(236)`
      background, kept as-is since it's an explicit, already-shipped
      harmonization with Terapi/jsoned's own status bar convention (see
      the "2-line status bar" entry above), not something to unpick here.
      Named ANSI colors are mapped by the terminal emulator itself
      according to whatever scheme it's configured with (light, dark,
      Solarized, ...), so "respecting terminal colors" — the roadmap's own
      bar for this item — comes for free from that choice rather than
      needing an explicit theme-switcher Mycora manages itself; no config
      option was added, and none is planned unless a real gap shows up
      (e.g. the status band's fixed background actually looking wrong on
      some real terminal theme). Added a bit of color to the split-pane
      borders on request: tree = blue, body preview = magenta, backlinks
      = its existing default-idle/cyan-when-focused (unchanged) — chosen
      to avoid the colors already carrying other meaning elsewhere (cyan
      = "focused/active," yellow = confirmation prompts, red = errors,
      green = markdown code). Manually verified in tmux: tree and body
      panes showed distinct blue/magenta borders simultaneously, and the
      backlinks pane's existing cyan-on-focus behavior still worked
      alongside them.
      **Since extended** (2026-07-11, user-requested): the body preview
      pane's `Block` now has 1-column horizontal padding
      (`ratatui::widgets::Padding::horizontal(1)`) between its border and
      the rendered Markdown, discussed first as an exploratory question —
      continuous prose reads more cramped flush against a border than a
      short list row does, so this pane (the one that's mostly running
      text rather than list rows) got it first. Tree and backlinks
      deliberately stay flush for now, on request — same idea, kept open
      to apply there too rather than done everywhere at once. Manually
      verified in tmux: rendered Markdown body text started with a clear
      left margin instead of touching the border, while the tree and
      backlinks panes' list rows were unaffected.
- [x] Split-pane layout: tree + note body + backlinks (2026-07-10) — three
      columns in Normal/Insert/ConfirmDelete modes, fixed proportions
      (40/40/20) at the time. **Since made resizable**: interactive
      resizing was deliberately kept as its own open item rather than
      folded in here (confirmed with the user before implementing), and
      landed later the same day — see the "Resizable panes" entry below.
      The body pane was a read-only plain-text preview of the selected note
      at the time (Markdown rendering was the separate item right below,
      resolved later the same day too) — it doesn't reuse `Mode::EditBody`'s
      full-pane overlay, which stays exactly as it was; pressing `e` still
      takes over the whole screen rather than editing in-place. The
      backlinks pane is similarly read-only and passive: it follows the
      current selection but doesn't reindex first (same as the link-count
      badges), and jumping to one of its entries still goes through the
      existing interactive `b` overlay (`Mode::Backlinks`) rather than
      being merged into the pane itself — a deliberate scope cut, agreed
      with the user before implementing, to keep this pass to "show a
      third pane" rather than "rebuild backlinks navigation." (**Since
      superseded**: the "Interactive backlinks pane" item below did
      exactly that merge, later the same day.) Manually
      verified in tmux: selecting a different note updated both the body
      and backlinks panes live, and all three full-pane overlays (search,
      the backlinks picker, the body editor) still take over the whole
      screen exactly as before rather than showing the split
      **Since fixed** (2026-07-11, user-reported): none of the panes
      actually scrolled — the user asked whether this had been verified,
      and it hadn't. Confirmed live against a 40-leaf-note generated test
      vault in a 15-row terminal: moving `j` past the tree pane's visible
      rows changed the breadcrumb (selection genuinely moved) but the
      pane kept showing the exact same rows, selected row fully
      off-screen; a note with `### Tasks`/`### Related` sections beyond
      the body preview's height was silently truncated with no way to
      see the rest. Root cause: every list pane (`draw_tree`,
      `draw_backlinks_pane`, `draw_search`, `draw_tag_results`,
      `draw_tag_list`) built a plain `ratatui_widgets::List` and rendered
      it with `render_widget` — never `render_stateful_widget` with a
      `ListState`, so ratatui had no "keep the selected item visible"
      behavior at all, just always rendered from the first item.
      Verified directly against the vendored `ratatui-widgets-0.3.2`
      source (`list/rendering.rs`'s `get_items_bounds`) before assuming
      a fix: `List`'s stateful render recomputes the visible window from
      `state.selected`/`state.offset` on *every* call, so a fresh
      `ListState` built each frame (`offset` starting at 0) still
      produces the correct scrolled window — no new persisted scroll
      state needed in `App` for these 5 panes, just switching each to
      `render_stateful_widget` with `ListState::default()
      .with_selected(selected_index)` (the backlinks pane only when
      `Mode::Backlinks`-focused, matching its existing focused-only
      highlight logic). The body preview (`Paragraph`) has no such
      built-in behavior — no "selected line" concept for prose — so it
      got real new state: `App::body_scroll: u16`, `Ctrl+d`/`Ctrl+u`
      (vim's half-page-scroll keys, Normal-mode-only, same scoping as
      `[`/`]`/`{`/`}`) adjust it by a fixed step, and a new
      `App::set_selected` became the *only* place `self.selected` is
      ever written (replacing 15 scattered direct assignments across
      `app.rs`) so `body_scroll` resets to 0 on every selection change
      in exactly one place rather than needing to remember it at each
      call site. Deliberately no upper clamp on scrolling down —
      computing the true max would mean `App` duplicating
      `markdown.rs`'s render+wrap logic just to count lines; overscroll
      just shows blank space and recovers with `Ctrl+u`. Manually
      verified in tmux re-running the exact scenario that surfaced the
      bug: the tree pane now visibly scrolls to keep the selected row on
      screen (confirmed via `tmux capture-pane -e`, the `[7m` reversed
      code lands on the correct row); `Ctrl+d` revealed the rest of a
      truncated note, `Ctrl+u` scrolled back, and selecting a different
      note reset to the top; `/` search results scrolled correctly
      moving `Down` past the fold; a normal-size terminal with short
      content rendered identically to before (no regression).
- [x] Resizable panes for the split layout above (2026-07-10) — `[`/`]`
      shrink/grow the tree pane, `{`/`}` shrink/grow the backlinks pane,
      always active in Normal mode (no dedicated resize mode — confirmed
      with the user before implementing: simpler, no new `Mode` variant or
      "which boundary is active" state to track). The body pane is never
      resized directly; it's the middle column, so it just absorbs
      whatever width the other two give up or take
      (`App::resize_pane`/`PANE_STEP_PCT` = 5, `PANE_MIN_PCT` = 10, floor
      applies to whichever of the two panes involved would cross it).
      Originally in-memory only, not persisted — a deliberate initial
      scope cut, confirmed with the user, since pane widths are a display
      preference rather than per-vault navigation state the way
      `selected`/`expanded` are. **Since persisted** (2026-07-10, on
      request): `Session`'s `pane_widths: Option<[u16; 3]>` is
      vault-agnostic (unlike the per-vault `selected`/`expanded` entries),
      since only one vault is ever navigable at a time and the layout
      applies regardless of which one that is. Saved at the same shutdown
      point as everything else in `Session` (`App::save_session`), and
      restored in `App::new` with validation (must sum to 100, no pane
      below `PANE_MIN_PCT`) so a hand-edited or stale session file can't
      hand `ui.rs` a layout it can't render sanely — falls back to the
      40/40/20 default if validation fails or nothing was ever saved.
      Manually verified in tmux: `]`/`{` visibly resized the tree/
      backlinks panes (tree wider, backlinks down to its floor), `q`/`q`
      quit, the saved `session.toml` showed the new widths, and
      relaunching restored the exact same layout.
- [x] Interactive backlinks pane (2026-07-10) — `b` no longer opens a
      separate full-screen overlay (`Mode::Backlinks` used to); it shifts
      keyboard focus onto the already-visible backlinks pane instead:
      `j`/`k` (or `Up`/`Down`) move within it, `Enter` jumps (expanding
      ancestors so the target is visible, same as before), `Esc` or `b`
      again returns focus to the tree. The focused pane gets a cyan border
      and reversed-highlight on the current entry, matching the tree's own
      selection styling. Confirmed with the user before implementing:
      replace the overlay entirely rather than keep both — one interaction
      path, not two doing the same thing. Also dropped the reindex-on-open
      that `show_backlinks` used to do: the pane now reads
      `App::live_backlinks()` exactly like the passive pane already did,
      so `b` no longer forces a fresh reindex — consistent with the
      passive pane's existing "doesn't reindex first" contract rather than
      a special case for the interactive path. Manually verified in tmux:
      focusing showed the cyan border and highlighted the first entry,
      `j` moved to the second, `Enter` jumped to it (tree selection,
      breadcrumb, and body preview all updated, backlinks pane correctly
      went empty since nothing links to the destination), and `b` then
      `Esc` on a different note returned to Normal without changing the
      tree selection
- [x] Render note body as formatted markdown in the preview pane
      (2026-07-10) — `src/markdown.rs`'s `render(&str) -> Vec<Line>` walks
      `pulldown-cmark`'s event stream and builds styled ratatui lines
      directly (a small hand-rolled `Renderer` with a style stack, not a
      dedicated ratatui-markdown crate — evaluated `ratatui-markdown`
      (2026-07) and passed: too young then and pinned to ratatui ^0.29 vs.
      our 0.30; nothing changed that assessment). Note: this roadmap entry
      previously said pulldown-cmark was "already in the stack for
      wikilink extraction" — that was never true, `link.rs`'s wikilink
      parser is a hand-rolled bracket scanner with no dependency at all;
      `pulldown-cmark` is a new dependency added specifically for this
      item. Covers headings (color-coded by level), bold/italic, inline
      and block code (green), bulleted/numbered lists (including nesting
      depth and correct ordinal counting), blockquotes (dim+italic), and
      horizontal rules. Not interactive: links render as plain text, and
      `[[wikilinks]]` aren't CommonMark syntax so they render as literal
      bracketed text too — highlighting them specially is a separate,
      not-yet-scoped concern from "render the Markdown"
- [x] Command palette (`:` command mode, à la vim/helix) (2026-07-10) —
      `:` in Normal mode enters `Mode::Command`; the input replaces only
      the status bar's hint row (row 2), leaving the breadcrumb (row 1)
      visible underneath, same footprint as `ConfirmDelete`'s prompt
      rather than a full-pane overlay like Search/EditBody. Explained the
      concept to the user before implementing (vim/helix `:` commands)
      and confirmed the starting command set via `AskUserQuestion`:
      `:reindex`, `:tags <tag1,tag2,...>`, `:q`/`:quit` — chosen because
      all three expose functionality that already existed in the backend
      with no keybinding of its own (manual reindex, v0.4's tag
      filtering), rather than inventing new behavior. `:tags` only
      supports OR/Any semantics for now (`TagFilterOp::Any`, matches any
      of the listed tags) — no AND syntax exposed yet, a deliberate
      first-pass simplification noted in the method's doc comment.
      Matches open a new full-pane `Mode::TagResults` overlay (same
      interaction shape as Search: `j`/`k` move, `Enter` jumps and
      expands ancestors, `Esc` cancels) since a tag query is a fresh,
      unrelated result set rather than context tied to the currently
      selected note. Unknown commands and `:tags` with no matches report
      through the status bar rather than silently no-opping: added a new
      `last_message: Option<String>` field (cyan, non-error feedback like
      "reindexed N note(s)") alongside the existing `last_error` (red).
      `reindex_mounted`'s signature changed from a void return to
      `anyhow::Result<usize>` so `:reindex` can report success/failure
      explicitly; `begin_search`'s existing call site was updated to
      match on the `Result` instead of the error being silently absorbed.
      Manually verified in tmux against a scratch vault with two
      `lang`-tagged notes and one untagged note: `:reindex` showed
      "reindexed 3 note(s)"; `:tags lang` opened Tag results listing both
      matches, `j` then `Enter` jumped to and selected the right note;
      `:tags nope` showed "no notes tagged nope" with no mode change;
      `:bogus` showed "ERROR unknown command: bogus"; `Esc` mid-command
      returned to Normal without executing anything; `:q` quit the app
      cleanly. **Since extended** (2026-07-10): a help popup listing every
      recognized command now shows automatically for the whole duration
      of `Mode::Command`, rather than requiring a `:help` command of its
      own — the user's own suggestion when asked how they wanted it
      triggered ("`:` produces the popup, then you continue typing the
      command over it"). `App::COMMAND_REFERENCE` is a small
      `&[(syntax, description)]` array, the single source both
      `execute_command`'s dispatch and `ui.rs`'s `draw_command_help`
      popup read from (kept in sync by hand, not generated — only three
      entries). The popup is a small bordered box (`ui.rs`'s
      `popup_rect`, `Clear`-first so it reads as opaque) anchored to the
      bottom-center of the main area, directly above the status-bar row
      where the `:` input itself is being typed; static, not filtered by
      what's typed so far. Manually verified in tmux: pressing `:` showed
      the popup with all three commands listed, typing `reindex` and
      `Enter` continued to work normally with the popup visible the whole
      time, and it disappeared once the command executed and the mode
      returned to Normal. **Since extended again** (2026-07-10): added
      `:panes reset`, resetting the split layout to
      `App::DEFAULT_PANE_WIDTHS` (40/40/20) — the user asked about adding
      a `:search` command too (equivalent to `/`), which was talked
      through and deliberately skipped: `/` already has a direct,
      prominently-hinted keybinding, so a `:search` command would just be
      a second entry point for the same thing rather than exposing
      anything new, unlike every other command in the palette. `:panes
      reset` earns its place differently: once pane widths started
      persisting across restarts (see "Resizable panes" above), there was
      no way back to the default short of mashing `[`/`]`/`{`/`}` by eye
      or hand-editing `session.toml` — a real gap, not a redundant second
      path. Manually verified in tmux: resized panes with `]`/`{`, `:`
      showed `:panes reset` in the help popup, running it reported "pane
      widths reset to default" and the layout snapped back to 40/40/20,
      and `:panes` with no argument showed "ERROR usage: :panes reset".
      **Since extended a third time** (2026-07-11, user-requested): added
      `:tags list`, listing every distinct tag in the active vault
      (alphabetical, with each tag's note count) in a new `Mode::TagList`
      full-pane overlay — `Enter` on a tag filters by it, transitioning
      straight into the existing `Mode::TagResults` (same as typing
      `:tags <that-tag>` yourself), so you can pick a tag without already
      knowing or typing its exact spelling. The user also asked about
      live autocompletion while typing `:tags <partial>` — discussed via
      `AskUserQuestion` and deferred: it's meaningfully more work (cursor-
      position-aware word detection in `Command` mode's input, a live
      filtering suggestion popup, `Tab`-to-complete key handling) for a
      need `:tags list` already covers in practice, since it sidesteps
      typing the tag at all rather than assisting with typing it. New
      `Index::all_tags(vault_id) -> Vec<(String, i64)>`
      (`SELECT tag, COUNT(*) ... GROUP BY tag ORDER BY tag`) backs it,
      scoped to the active vault same as `filter_by_tags`. Extracted
      `command_tags`'s filter-and-open-`TagResults` logic into a shared
      `show_tag_results(tags)` so both `:tags <tag1,tag2,...>` and
      picking a tag from the list go through the same path. The literal
      argument `"list"` is checked before the comma-split filter logic —
      same minor, accepted edge case as `:panes reset`'s literal-argument
      dispatch (a tag actually named "list" needs `:tags list,list` or
      similar to reach via filtering). Manually verified in tmux against
      the showcase vault: `:tags list` showed every tag alphabetically
      with correct singular/plural note counts; selecting one and
      pressing `Enter` opened `Tag results` for just that tag, and
      `Enter` again jumped to and selected the matching note; `:tags`
      with no argument showed the updated usage message mentioning both
      forms.
      **Since extended a fourth time** (2026-07-12, user-reported): a
      status message set by a command (`:export`'s "exported to ...",
      `:reindex`'s "reindexed N note(s)", ...) used to stay in the hint
      row forever — nothing ever cleared `last_error`/`last_message`
      except another command overwriting them, so plain navigation after
      running a command left the hint row permanently stuck showing that
      command's result instead of falling back to the current mode's
      normal keybinding hints. New `App::clear_transient_status`, called
      once per keypress in `event::poll_and_handle` right before mode
      dispatch (same spot as the existing per-keypress
      `reset_quit_confirmation` call) — whatever the keypress itself
      does can still set a fresh message immediately afterward in the
      same call, so a command's own result still shows correctly the
      instant it runs; it just doesn't linger past the next keypress
      anymore. Manually verified in tmux: ran `:export <path>`, confirmed
      "exported to ..." showed in the hint row, pressed `j`, confirmed
      the row immediately reverted to Normal mode's usual keybinding
      hints instead of staying stuck.
- [x] Session state: remember last open note, expanded/collapsed branches
      (2026-07-10) — new `src/session.rs`: `Session::load`/`save` read and
      write `~/.local/share/mycora/session.toml` (XDG data dir alongside
      the SQLite index, since this is generated state, not user-authored
      config), keyed by vault name so switching which vault is `default`
      doesn't clobber another vault's remembered position. Saved once at
      shutdown (`App::save_session`, called from `main.rs` right after
      `run()` returns) rather than write-through on every expand/collapse
      or selection change — this is ephemeral navigation state, not user
      content, so per-keystroke disk writes would be wasted I/O for no
      benefit over saving once on exit. That single save point after
      `run()` naturally covers both `q`/`q` and `Ctrl+C`, since both just
      set `should_quit` and let the same loop-exit path handle the rest,
      with no special-casing needed for either. Restored in `App::new`:
      ids that no longer resolve (note deleted, vault changed) are
      dropped rather than kept dangling, and the restored selection's
      ancestors are always expanded to guarantee it's actually visible,
      regardless of what the saved expanded set had (extracted the
      existing `reveal`'s ancestor-walk into a free function,
      `reveal_ancestors`, since `App::new` needs it before `self` exists).
      Manually verified in tmux: collapsed a branch and selected a
      different root, quit with `q`/`q`, relaunched, and both were
      restored exactly; repeated with `Ctrl+C` instead of `q`/`q` and the
      session was saved there too
- [x] 2-line status bar, harmonized with Terapi/jsoned (2026-07-10):
      `Length(2)` band split into two `Length(1)` rows, `Color::Indexed(236)`
      background on both. Row 1 (`draw_breadcrumb`): `vault › branch › note`
      — `App::vault_name()` plus `App::breadcrumb_titles()` (ancestor
      titles from the selected note's root down to itself). Row 2
      (`draw_hint_row`): a cyan bold mode label, then hints tokenized on a
      `"key: label"` convention (`spans_from_hints`, double-space
      separated) into bold key / dim colon+separator / muted label spans
      — every mode's hint string was rewritten from
      `"j/k move  h/l fold"` to `"j/k: move  h/l: fold"` to fit that
      shape. The delete-confirmation prompt, the quit-confirm notice, and
      the last-error message still take over row 2 exactly as before
      (same precedence), just now with row 1's breadcrumb staying visible
      above them rather than being replaced too. Manually verified in
      tmux: breadcrumb correctly showed `default › Parent Note › Child
      Note` after navigating into a nested note, and the delete prompt
      left the breadcrumb in place while replacing only row 2
- [ ] No top-level Tabs bar for now — Mycora's single-view-with-panels
      layout (tree + editor + backlinks) matches jsoned's model, not
      terapi's multi-view one. Revisit only if a genuinely separate
      top-level view emerges (e.g. a tree view vs. a link/graph view).

## v0.8 — Import / export & interoperability

Goal: notes are never trapped in Mycora.

- [x] Import from an existing Obsidian-style vault (wikilinks +
      frontmatter) (2026-07-11) — the core tension raised and resolved
      with the user before designing further: Obsidian has no `parent`
      field at all, its only organizational structure is the filesystem
      (folders), and its notes form a free graph via `[[wikilinks]]`,
      not a strict tree the way Mycora requires. **Confirmed: map the
      folder structure onto the tree** — a subdirectory becomes a parent
      note (reusing a same-named `.md` file as that note's own content
      if one exists, synthesizing an empty placeholder if not), and
      everything inside it becomes children — rather than a flat import
      that would discard the user's actual organization entirely.
      New `src/import.rs`, mirroring `Vault::load`'s own `(Tree,
      warnings)` shape but reading a foreign directory format:
      `import_obsidian_vault(source)` walks recursively (skipping
      `.obsidian/` and anything that isn't `.md` — images, canvases,
      plugin data), builds every note via `Tree::insert_loaded` (the
      same bulk-load path `Vault::load()` itself uses for "construct
      many notes then derive structure") with a fresh `NoteId::new()`
      each, then one `Tree::rebuild_hierarchy()` call at the end. Per
      file: **title** comes from the filename stem, not a leading `#
      Heading` the way Mycora's own format expects — Obsidian doesn't
      reliably have one. **Frontmatter** gets its own loose parse (a
      small local `tags: Option<TagsField>` type, `#[serde(untagged)]`
      over a single string or a list, since Obsidian accepts either
      shape) rather than reusing `vault.rs`'s strict `Frontmatter`
      struct, which would reject anything not matching Mycora's own
      exact `id`/`parent`/`order`/`tags`/`created`/`updated` schema;
      missing or malformed frontmatter is "no tags," not an error, same
      self-heal-and-warn stance `vault.rs` already takes. **Body**: every
      `[[Title|Alias]]` or `[[Title#Heading]]` is rewritten down to
      plain `[[Title]]` — a small hand-rolled scanner in the same style
      as `link.rs`'s own wikilink extraction — since Mycora's scanner
      only understands bare `[[Title]]`; without this, every aliased or
      heading-anchored link (extremely common in real Obsidian vaults)
      would silently become a broken link the moment it's resolved,
      undercutting a good chunk of the point of importing at all.
      CLI-only: `mycora import <source> <name> <path>`, mirroring
      `vault init`'s exact shape (always creates a *new* vault, always
      mounts it, registers via the same `Config::add_vault`) — no
      TUI-side `:import`, since unlike `:export` (which reads the
      already-selected note) an import has no analogous "current
      selection" to import *into*; it's inherently a create-a-new-vault
      operation, the same reason `vault init` itself is CLI-only.
      Refuses if the destination path already exists and is non-empty,
      same don't-silently-clobber instinct as `export`'s
      refuse-on-existing-file and `vault add`'s refuse-on-duplicate-name.
      10 unit tests in `import.rs` (alias/heading-anchor stripping,
      frontmatter splitting, a flat vault, folder-note reuse, synthesized
      empty folder notes, skipping `.obsidian/`). Manually verified
      end-to-end against a hand-built scratch Obsidian vault (a top-level
      note, a `Projects/` folder with a matching `Projects.md` plus a
      child note, an `Archive/` folder with *no* matching note, one
      `[[Title#Heading]]` link and one `[[Title|Alias]]` link, one note
      with list-form `tags: [work, active]`, one with single-string
      `tags: urgent`, one with no frontmatter at all): the imported
      tree's shape in the TUI matched the source folder layout exactly
      (`Projects` kept its own content *and* got the child note nested
      under it; `Archive` appeared empty but present with its child);
      both link forms rendered down to plain `[[Title]]` and `mycora
      reindex` resolved them with zero broken-link warnings; both tag
      forms came through as real Mycora tags; re-running the same import
      against the same now-populated destination refused rather than
      duplicating or overwriting anything.
- [x] Export a subtree to a single flattened Markdown document
      (2026-07-11) — both surfaces landed together, confirmed with the
      user before implementing: the TUI's `:export <path>` (exports the
      *selected* note's subtree — a read operation, not gated by
      `require_editable`, so it works on a read-only mounted vault's
      note just as well as the active vault's) and a CLI
      `mycora export <title> <output>` (title-matched within the active
      vault only, since a headless invocation has no selection context
      to disambiguate with — errors on zero or multiple matches rather
      than guessing, same "don't silently guess" instinct as ambiguous
      wikilink resolution (see v0.5's "fan-out" entry above) and
      `vault promote`, pointing at the TUI's `:export` for the
      disambiguation-by-direct-selection case). New `src/export.rs`,
      pure and `Tree`-only (no I/O, no `App`/`Vault` dependency): `Note`
      titles become headings at a level matching depth within the
      exported subtree (the root note is `#`, its children `##`, ...),
      and any ATX headings already inside a note's own body are shifted
      deeper by that same amount via a line-by-line scan (checking for
      the CommonMark-required trailing space/end-of-line after 1-6 `#`
      characters, so e.g. `#nothashtag` isn't mistaken for a heading) —
      same "small hand-rolled scanner, no full parse" spirit as
      `link.rs`'s wikilink extraction — so a note's own internal
      structure nests correctly under its title instead of competing
      with it. Confirmed two more things with the user up front:
      **refuses if the output path already exists** rather than
      overwriting it (Mycora has zero visibility/safety-net — no trash,
      no undo — for a path outside a vault, unlike notes), and no
      frontmatter or `[[wikilink]]` rewriting in this first pass
      (wikilinks stay literal text, same as the body preview already
      renders them — converting ones that resolve to another note *in
      the same export* into working Markdown anchors is a plausible v2,
      deliberately not attempted now). 6 unit tests in `export.rs`
      (single leaf, nested depth, heading-shift correctness, the
      not-a-heading edge case, empty body, subtree isolation from
      unrelated siblings). Manually verified: CLI export of a two-level
      subtree produced correctly nested headings end to end; re-running
      against the same output path refused with "already exists"; a
      vault with two notes sharing a title correctly refused ambiguity
      and named `:export` as the way to disambiguate; the TUI's
      `:export <path>` on the same note produced byte-identical output
      to the CLI path; `:export` with no argument showed the usage
      error.
- [x] Export a subtree to a PDF file (2026-07-11) — user-requested
      (2026-07-10), built directly on top of the Markdown export above:
      `flatten_subtree` produces the same Markdown either way, so PDF
      export is "that Markdown, rendered to a page layout" rather than a
      separate pipeline. Two forks confirmed with the user before
      implementing:
      - **Rendering approach**: a pure-Rust crate over shelling out to
        `pandoc`/`wkhtmltopdf` — those need a whole LaTeX toolchain or a
        largely-unmaintained HTML-to-PDF binary preinstalled, which isn't
        reliably true on the user's machine, vs. a self-contained
        dependency that behaves identically everywhere. Landed on
        [`markdown2pdf`](https://crates.io/crates/markdown2pdf) (checked
        the actual crate source, not just its docs, before committing to
        it): actively maintained, edition 2024, takes Markdown straight
        in and a page-laid-out PDF straight out (headings, bold/italic,
        code blocks, lists, links), pinned to a current `printpdf 0.9`
        rather than the years-stale `genpdf`/forks (last released 2021,
        pinned to `printpdf 0.3.4`) that were the other realistic
        pure-Rust option. Its `fetch`/`svg` cargo features (network
        image fetch, SVG rasterization) are both off — Mycora doesn't
        need either and left them opt-in.
      - **Command surface**: extend `:export`/`mycora export` rather
        than add a dedicated command — the output *path*'s extension
        decides the format (`.pdf` renders through `markdown2pdf`,
        anything else is written as plain Markdown, same as before), so
        there's nothing new to learn or document as a separate command
        surface, just one more extension the existing one understands.
      New `export::write_output(content, path) -> Result<(), String>`
      (`export.rs`) is the single place both the TUI's `:export` and the
      CLI's `mycora export` now call to actually write the file, sharing
      the extension check (`is_pdf_path`) between them instead of
      duplicating the branch at each call site. 4 new unit tests
      (extension detection case-insensitivity, a non-`.pdf` path writes
      Markdown verbatim, a `.pdf` path produces bytes starting with the
      real `%PDF-` magic number). Manually verified: CLI `mycora export
      <title> out.pdf` and the TUI's `:export out.pdf` on the same note
      both produced a valid PDF (`file` reported "PDF document, version
      1.5"; `pdftotext` round-tripped the heading hierarchy, paragraphs,
      and list items back out correctly); re-running against the same
      `.pdf` path refused with "already exists", same as the Markdown
      path already did.
- [ ] Optional Postman/Terapi-style templating hooks (stretch — evaluate
      whether this belongs here or in a separate tool)

## v0.9 — Hardening

Goal: stability before a public release.

- [x] Test coverage on tree operations (especially move/copy/delete edge
      cases) and link integrity (2026-07-12) — audited `tree.rs`,
      `link.rs`, `vault.rs`, and `index.rs` against their own operations
      for untested edge cases before adding anything; `index.rs`'s
      link-integrity coverage (fan-out, broken links, self-links,
      cross-vault resolution/backlinks) was already thorough, so this
      focused on `tree.rs` and `link.rs`, where the gaps actually were.
      19 new tests: `move_note` to root, a same-parent no-op, appending
      after existing children, a direct-child (not just grandchild)
      cycle rejection; `move_up`/`move_down` at the opposite untested
      boundary, at root level (siblings tests only exercised the
      under-a-parent path before), and on a missing note; `deep_copy` on
      a missing note, and two claims its own doc comment made but no
      test actually checked — that tags carry over and that timestamps
      are freshly stamped, not cloned from the original; `delete_subtree`
      removing a root from `roots()`, preserving untouched siblings, and
      the returned `Vec<(NoteId, Note)>` actually holding each removed
      note's own title/body (not just the right ids) — that data is what
      a caller would restore from on undo, so its correctness matters as
      much as the ids; a direct `subtree_ids` ordering test; three
      `link.rs` cases (adjacent wikilinks with no separator, a stray
      `]]` before the first real link, and a pinning test for the
      documented nested-bracket "naive scanner" quirk already called out
      in this file's own module doc comment and CLAUDE.md, which nothing
      had actually locked in before). Auditing `rebuild_hierarchy`
      surfaced a real, if narrow, bug along the way: a note whose
      `parent` field names itself (unreachable through any in-app
      mutation — `move_note`'s cycle check already refuses it — but
      producible by hand-edited on-disk frontmatter) became its own sole
      child and never appeared in `roots()`, silently unreachable from
      any real traversal despite still existing in the tree. This broke
      the self-healing contract every other malformed-parent case already
      gets (see [[Markdown as source of truth]] in the showcase vault),
      so fixed it the same way: treated like any other unresolvable
      parent, promoted to root with a warning, self-healed on next save.
      One `tree.rs` unit test pins the fix directly; a `vault.rs`
      integration test (hand-crafted on-disk frontmatter with
      `parent: <own id>`) confirms the same self-heal end to end through
      a real `Vault::load()`. Manually verified: `mycora reindex` against
      a hand-crafted self-parented note file printed the same "parent not
      found, promoted to root" warning and rewrote the file with
      `parent: null`; a full TUI session (reorder, copy, delete the copy)
      against a generated test vault showed no regression.
- [x] Crash-safety: no data loss on unexpected exit (atomic writes)
      (2026-07-12) — audited every persistent-state write path in the
      crate for atomicity before touching any of them. `vault.rs`'s note
      writes were already atomic (temp file + `fs::rename`, since v0.2);
      `config.rs`'s `write_raw` and `session.rs`'s `Session::save` were
      not — both used a plain `fs::write`, so a crash or power loss
      mid-write could leave `config.toml` or `session.toml` truncated or
      corrupted on next load. Fixed both with the same temp-file+rename
      pattern `vault.rs` already used (`path.with_extension("toml.tmp")`,
      write there, then `fs::rename` onto the real path — a rename is
      atomic on the same filesystem, so the real file is either the old
      complete content or the new complete content, never a partial
      write). Deliberately left `export.rs`'s `write_output` alone — an
      export target is an arbitrary user-chosen path outside any vault,
      already guarded by refuse-if-exists, and not persistent Mycora
      state, so a failed write there just means "retry the export," not
      data loss the way a corrupted `config.toml`/`session.toml` would
      be. The SQLite index was also left alone: it's explicitly
      disposable (see `Disposable SQLite index`) and `mycora reindex`
      rebuilds it from scratch at any time, so partial-write corruption
      there is a non-issue by design, not a gap. 2 new unit tests
      (`config.rs`, `session.rs`) assert no `*.toml.tmp` file is left
      behind after a save. Manually verified end to end in a scratch
      `HOME`: a full TUI session (navigate, quit) produced a clean
      `session.toml` with no leftover `.tmp`; `mycora vault add`
      produced a clean `config.toml` the same way.
- [x] Large-vault performance pass (thousands of notes) (2026-07-12) —
      measured before touching anything (see [BENCHMARK.md](./BENCHMARK.md)
      for the full methodology and numbers), same "benchmark before
      committing" instinct already applied to the tantivy decision above:
      a new `examples/benchmark.rs` times cold `Vault::load`, a full
      `Index::reindex`, an `Index::search`, and `App::visible_rows`
      (worst case, every note expanded) against synthetic vaults from
      100 to 10,000 notes. Three of those four scaled linearly and were
      never a problem, including `visible_rows` — the one CLAUDE.md
      itself flagged as "recomputed every call... revisit if it shows up
      in profiling," and it didn't. `reindex` was the outlier: 104
      seconds at 10,000 notes, growing quadratically (2× the notes took
      12× the time). Root cause: `Index::write_links` resolves every
      `[[wikilink]]` title via `WHERE title = ?1`, and `notes` had no
      index covering `title` — every lookup was a full table scan of an
      *O(N)*-row table, done *O(N)* times. Fixed with
      `CREATE INDEX IF NOT EXISTS idx_notes_title ON notes(title)` (an
      old on-disk `index.sqlite3` picks it up for free next time it's
      opened, same as any other `IF NOT EXISTS` — the disposable-index
      philosophy means no real migration needed) plus switching
      `write_links`'s per-iteration `tx.prepare` to `tx.prepare_cached`
      (identical SQL text every loop iteration, no reason to recompile
      it each time). Result: 10,000-note reindex went from 104.28s to
      311.7ms, a ~335× speedup, now scaling linearly like everything
      else. Cross-checked outside the isolated benchmark too: a real
      `mycora reindex` against a 5,000-note vault (generated by the
      existing `examples/generate-test-vault`, not the new benchmark's
      own generator) completed in 0.305s wall-clock, and `/` search in
      the TUI against that vault showed no perceptible delay.
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
  **Since extended** (2026-07-10, user-requested): `mycora vault add
  <name> <path> [--no-mount]` registers a new entry in `config.toml`'s
  registry from the CLI, rather than hand-editing the TOML — still no
  runtime mount/unmount (that's still config-file-and-relaunch, as
  above), just a friendlier way to add an entry to begin with.
  `Config::add_vault` rewrites the whole file from a fresh parse (like
  `cargo add` rewriting `Cargo.toml`), migrating a legacy `vault_path`
  key into an explicit `"default"` entry first if that's all that was
  there, and rejecting a duplicate name outright rather than silently
  overwriting it. Manually verified: adding a vault to an empty/missing
  config created it; adding a second preserved the first; adding a
  duplicate name errored without touching the file; adding to a
  `vault_path`-only config correctly migrated it alongside the new entry.
  **Since extended again** (2026-07-10, user-requested): `mycora vault
  init <name> <path>` creates the vault directory (`Vault::open`'s usual
  lazy `create_dir_all`), registers it always-mounted (reuses
  `Config::add_vault`), then reports whether it actually became the
  active/editable vault — it only does if it ends up named `"default"`
  (or is the sole/first mounted entry), per `Config::active_vault`'s
  existing rule. Raised an ambiguous case explicitly with the user before
  implementing (`AskUserQuestion`): what happens when a `"default"`
  vault already exists? **Confirmed: create and mount it anyway,
  report honestly that it's staying read-only and why, and never
  silently rename/reassign the existing `"default"` entry to make
  room** — the other two options considered (auto-demote the existing
  default, or refuse to create anything at all) were both rejected as
  either too surprising or too inconvenient. Manually verified: `vault
  init default <path>` into an empty config became the active vault
  (confirmed live in the TUI: `default` was the editable tree, its
  breadcrumb showed the auto-seeded "Welcome to Mycora" note);
  `vault init work <path>` afterward printed the "stays read-only"
  message, and relaunching the TUI showed `work` stacked read-only below
  `default`, exactly as the message said it would.
  **Since extended a third time** (2026-07-10, user-requested): `mycora
  vault rename <old> <new>` renames a registry entry in place (path/
  mounted untouched; a no-op if old == new; errors if `old` isn't
  registered or `new` is already taken), and `mycora vault promote
  <name>` makes a vault active by renaming it to `"default"` — the exact
  name `Config::active_vault` looks for, so this is also how the "stays
  read-only" case from `vault init` above gets resolved afterward.
  `promote` is deliberately narrow: raised the same "what if `default`
  already exists" question again before implementing, and this time
  **confirmed the opposite answer from `init`'s**: `promote` *refuses*
  outright if a different vault already holds `"default"`, rather than
  auto-swapping names — the error message tells you to `vault rename
  default <new-name>` first, then retry. `promote` is implemented as a
  thin wrapper that ends up renaming the target to `"default"`, sharing
  `rename_vault`'s read/write plumbing (both go through new private
  `read_raw`/`write_raw`/`migrate_legacy_vault_path` helpers, refactored
  out of `add_vault` at the same time so all four `*_vault` methods share
  one implementation of "parse, mutate, rewrite the whole file"). Both
  new methods are no-ops if there's nothing to do (renaming to the same
  name; promoting an already-`"default"` vault). Manually verified: with
  two vaults mounted, `promote` on the non-default one failed with the
  expected message and touched nothing; `rename default old-default`
  freed up the name; `promote work` then succeeded, and relaunching the
  TUI confirmed `work`'s content was now the editable tree (its own
  auto-seeded "Welcome to Mycora" note) with `old-default` stacked
  read-only below it.
  **Since extended a fourth time** (2026-07-10, user-requested): `mycora
  vault mount <name>`/`vault unmount <name>` toggle a registered vault's
  `mounted` flag directly (thin wrappers over a shared private
  `Config::set_mounted`), each a no-op if the flag's already set that
  way. Implementing these surfaced a **pre-existing latent panic** in
  `App::new`, unrelated to this change but made trivially reachable by
  it: `Config::active_vault`'s self-heal (its own doc comment: "the app
  always needs at least one editable vault") returns *some* vault even
  when every registry entry has `mounted = false`, but that self-healed
  pick isn't necessarily itself in `mounted_vaults()` — `App::new` only
  loaded vaults from `mounted_vaults()`, so its `primary_idx` lookup for
  the self-healed `active` vault would find nothing and hit an
  `.expect(...)`. Previously only reachable by hand-editing every entry
  in `config.toml` to `mounted = false`; `vault unmount`ing your only
  vault (or every mounted one) makes it trivial. **Fixed alongside this
  feature** rather than shipped as a companion bug: `App::new` now
  explicitly includes `active` in the set of vaults it loads even when
  `active` itself isn't flagged `mounted`, so the self-heal promise
  actually holds end-to-end instead of panicking one level up from where
  it was made. Manually verified two scenarios against a two-vault
  registry: unmounting just `default` (leaving `archive` mounted) loaded
  `archive` as active with no panic; then unmounting `archive` too, so
  *every* entry was `mounted = false`, still loaded `default` cleanly
  via the self-heal path instead of crashing.
  **Since extended a fifth time** (2026-07-10, user-requested): `mycora
  vault remove <name>` and `mycora vault list`. Discussed `remove`'s
  semantics with the user up front before implementing, rather than
  guessing: **it only ever unregisters the `config.toml` entry, never
  touches the vault's files on disk** (consistent with notes being the
  source of truth and the registry being just a pointer to them — the
  same instinct behind `Vault::trash_note` never permanently deleting a
  note either), and **it refuses outright on `"default"`**, erroring
  with the exact fix (`vault rename default <new-name>`, or `vault
  promote <other-name>` to take over the name first) rather than
  allowing the active vault to be silently unregistered. `vault list`
  reads through `Config::load()` (not a raw file dump) so it reflects
  the same self-healed/legacy-migrated view every other command and the
  TUI itself see; each entry shows its path plus `[active, mounted]`-style
  tags. Manually verified: `vault list` correctly tagged the active vault
  among three registered ones; `vault remove default` refused with the
  documented message; promoting a different vault to `"default"` first
  (freeing the old name via `rename`) then let the old entry be removed
  under its new name, confirmed via `vault list` showing one fewer entry
  and the removed vault's on-disk note file still present and unchanged
  afterward.
  **Since extended a sixth time** (2026-07-10, user-requested): read-only
  mounted vaults are now fully navigable — `j`/`k` continue past the
  active vault's last row into each read-only vault's section instead of
  stopping at the boundary, `l`/`Space` expand/collapse branches *inside*
  a read-only vault (previously roots-only, always collapsed), the body
  preview and backlinks pane both work for whatever's selected, and the
  breadcrumb shows the vault actually being looked at. Everything about a
  read-only vault stays read-only — no create/rename/delete/move/
  reorder/copy/edit-body — reporting "this vault is read-only" through
  the status bar rather than silently no-oping.
  Auditing `app.rs` before writing any of this turned up two real bugs
  the change would otherwise trigger or leave latent, both fixed as part
  of this pass: `create_child`/`create_sibling` had no guard against
  `selected` pointing outside `self.tree` at all — once `selected` can
  point into a read-only vault, `create_child` would have silently
  created a stray, wrongly-parented note *in the active vault*, since
  `Tree::create_note` doesn't itself validate that a parent id exists;
  and `breadcrumb_titles`/`vault_name` were hardcoded to the active
  vault's tree/id, which would have shown the active vault's name with
  an empty path while actually browsing a read-only one. Every other
  mutating method (`copy_selected`, `indent_selected`, `outdent_selected`,
  `reorder`, `begin_rename`, `begin_edit_body`) already no-opped safely
  via existing `self.tree.get(id)` checks, but silently; `request_delete`
  had no guard (would've opened the confirm-delete prompt for an
  unremovable note). All nine now share one `require_editable` check.
  Implementation: a new `resolve(id) -> Option<(&Tree, &str)>` (checks
  the active tree, else scans the read-only ones) backs every read
  accessor that must work regardless of which vault the selection is in;
  a new `TreeRow` enum (`Note { .. }` / `VaultSeparator`) and
  `visible_rows()` replace the old active-only `visible_notes()` +
  roots-only `other_vault_sections()` with one combined, depth-first,
  fully-navigable list `move_selection` and `ui.rs`'s tree rendering both
  consume. `reveal()` (expands ancestors before a search/backlinks jump)
  needed direct field-disjoint access to `self.tree`/`self.other_vaults`
  vs. `self.expanded` rather than going through `resolve(&self)`, since
  it mutates `expanded` while needing a live tree reference at the same
  time — same reason the existing free-function `reveal_ancestors` isn't
  a method either. Deliberately didn't add a "(read-only)" badge anywhere
  at first — the corrected breadcrumb (showing the real vault name) plus
  the tree pane's dimmed read-only rows seemed like enough signal without
  more UI surface. Manually verified end-to-end in tmux against a
  two-vault scratch setup (active vault with a nested branch, read-only
  vault with its own nested branch and a note targeted by a cross-vault
  wikilink from the active vault): `j`/`k` crossed the boundary into the
  read-only section; `l` expanded a collapsed branch inside it, revealing
  a nested note; selecting that note showed its body and, in the
  backlinks pane, the active vault's note that links to it; `b` then
  `Enter` on that backlink jumped back into the active vault, breadcrumb
  updating correctly both directions; every edit key (`a`, `d`, `i`, `e`,
  `y`, `Tab`, `K`, `J`) on the read-only note showed "this vault is
  read-only" and left both vaults' files byte-identical on disk
  (confirmed via `md5sum`) — critically, `a` did *not* leak a stray note
  into the active vault; the same keys still worked normally afterward
  against a note in the active vault.
  **Since extended a seventh time** (2026-07-10, user-requested): the
  "deliberately didn't add a badge" call above got revisited — a
  `READ-ONLY` label now sits right-aligned on the breadcrumb row (row 1
  of the status bar) whenever the selection is read-only, via a new
  `App::selected_is_read_only()` and a fixed-width
  (`READ_ONLY_MARKER_WIDTH = 12`) right-hand column in `ui.rs`'s
  `draw_breadcrumb`, split off from the breadcrumb text with a `Layout`
  the same way the rest of the split-pane UI already does. Fixed-width
  rather than only-as-wide-as-the-text so the breadcrumb's own column
  doesn't shift width as you move in and out of read-only vaults; blank
  (but still painted with the status bar's background) when editable, so
  the row's background stays a solid, unbroken band either way. Styled
  dim/italic gray rather than a louder color, to match the tree pane's
  existing read-only dimming rather than introducing a new "this needs
  your attention" color on top of the established
  red=error/yellow=confirm/cyan=focused palette. Manually verified in
  tmux: the marker appeared exactly when a read-only note was selected
  and disappeared exactly when selection returned to the active vault,
  with the breadcrumb's own text never shifting width.
  **Since extended an eighth time** (2026-07-10, user-requested):
  Normal mode's hint row (row 2) now dims the seven mutating hints —
  `a/o: new`, `y: copy`, `Tab/S-Tab: move`, `K/J: reorder`, `i: rename`,
  `e: edit`, `d: delete` — down to the same style as the row's own
  separators whenever `App::selected_is_read_only()` is true, rather
  than showing every hint at full brightness even though pressing one of
  those seven would immediately bounce off `require_editable` and show
  "this vault is read-only." `u: undo`/`^R: redo` deliberately stay
  full-brightness — they aren't gated by `require_editable` at all
  (the undo stack can never hold a foreign-vault action, so both always
  work regardless of what's selected), so dimming them would have been
  inaccurate, not just extra caution. `spans_from_hints` gained a
  `disabled_keys: &[&str]` parameter (matched against each token's exact
  key substring, e.g. `"Tab/S-Tab"`), passed as a hardcoded list from
  `draw_hint_row` only when `Mode::Normal` and the selection is
  read-only — every other mode's hints are either already non-mutating
  or only ever reachable with an editable selection to begin with, so no
  dimming logic was needed there. Manually verified in tmux (ANSI codes
  inspected via `tmux capture-pane -e`): on the active vault's note, all
  seven hints carried the normal bold-key/muted-label styling; on a
  read-only note, those same seven rendered with no bold/color codes at
  all (fully dimmed), while `j/k`, `u`, `^R`, `/`, `b`, and the resize
  keys kept their normal styling throughout.
  **Since extended a ninth time** (2026-07-11, user-reported): the CLI's
  own `mycora --help`/`mycora reindex --help` text still said "the
  active vault" — stale since `reindex` started covering every mounted
  vault back in the "Since extended" note above. Fixed the doc comment
  driving clap's generated help, split into a short summary (blank-line
  paragraph break makes clap show it in `mycora --help`'s command list
  and `reindex -h`) plus a longer explanation for `reindex --help`.
  `mycora vault --help` and every subcommand's own `--help` were already
  accurate and complete, checked at the same time — no changes needed.
  **Since extended a tenth time** (2026-07-12, user-requested): an
  unmounted registered vault used to be entirely invisible in the TUI —
  the only way to know it existed was `mycora vault list` from the
  shell, or remembering `config.toml`'s contents. Now every unmounted
  entry gets its own single, unexpandable placeholder row in the tree
  (`⊘ name`, `Color::DarkGray`, no fold marker — confirmed the exact
  icon/color with the user via a side-by-side preview before
  implementing, see `AskUserQuestion`'s options), appended after every
  mounted vault's section. Selecting it shows the vault's path and the
  exact `mycora vault mount <name>` command to bring it back, in the
  body preview, instead of a note body; the breadcrumb's corner marker
  reads `UNMOUNTED` instead of `READ-ONLY`; every mutating hint dims out
  same as a read-only note's does, plus `h`/`l`/`Space` (fold) on top of
  that, since there's nothing loaded to expand at all. `App::selected`
  can't represent this (there's no `NoteId` — nothing is loaded), so a
  new `selected_unmounted_vault: Option<String>` field holds it instead,
  mutually exclusive with `selected` via the existing `set_selected`
  choke point plus a new `set_selected_unmounted_vault` mirroring it —
  deliberately *not* a bigger `Selection` enum replacing `selected`
  outright, which would have rippled through every one of the ~15
  existing call sites that already assume a bare `Option<NoteId>`, for
  a purely additive feature that doesn't need it.

  Auditing every mutating command for how it behaves when `self.selected`
  is `None` (now reachable just by navigating onto an unmounted vault's
  row, not only by deleting the very last note in an otherwise-empty
  vault, the one existing way to reach it before) surfaced a real latent
  bug: `create_sibling` used `if let Some(id) = self.selected && \
  !self.require_editable(id) { return; }`, which only returns early when
  something selected turns out not to be editable — with nothing
  selected at all, that condition is false and it fell straight through,
  silently creating a new *root-level* note in the *active* vault instead
  of doing nothing. Fixed to the same `let Some(id) = self.selected else
  { return };` shape every other mutating command already used.

  Manually verified end to end in tmux: launched against a config with
  one mounted and one unmounted vault, confirmed the `⊘ old-notes` row
  renders correctly; selecting it showed the exact path and mount
  command in the body preview, `UNMOUNTED` in the breadcrumb; `tmux
  capture-pane -e` confirmed `h/l/space` through `d: delete` render
  fully dimmed while `j/k`, `u`, `^R`, `/` keep normal styling; pressing
  `l`/`o`/`a`/`d` on that row each did nothing at all (no note created,
  no delete prompt, no navigation change) — confirming the
  `create_sibling` fix; `j`/`k` correctly wrapped through the row in
  both directions.
- [x] **Archived vaults**: `mycora vault archive <name>` / `mycora vault
      unarchive <name>` (2026-07-12) — following on from unmounted vaults
      becoming visible (see "Multiple vaults" above), two more vault
      states were floated: archiving (compress to a single file) and
      locking (encrypt at rest). Locking was **abandoned outright** on
      the user's own call, for the reason already raised when discussing
      it: passphrase handling, key derivation, and the consequences of
      getting authenticated encryption subtly wrong are a lot of security
      surface for a note-taking tool to own, with no recovery path if a
      passphrase is lost — a vault is just a directory of Markdown files,
      so it already encrypts cleanly with existing, audited tools outside
      Mycora entirely (LUKS, VeraCrypt, `age`).

      Archiving *was* built. CLI-only, same reasoning as every other
      `vault ...` subcommand: it acts on the registry, not on "whatever's
      currently open," so it doesn't belong in the `:` command palette
      (see "Command palette" above) any more than `vault add`/`vault
      remove` do. Two forks confirmed via `AskUserQuestion` before
      writing any code: **tar.gz** (new `tar`/`flate2` dependencies, both
      pure-Rust — `flate2` defaults to its `miniz_oxide` backend, no
      system zlib needed) over `zip`, since a vault is many small,
      textually similar Markdown files — a solid, streaming-compressed
      format like tar.gz genuinely compresses that shape of data better
      than zip's per-file-independent compression, and tar+gzip is what
      Cargo itself uses for exactly this "package up a directory" case;
      and **delete the original after verifying the archive**, not
      "keep both" — `vault archive` re-opens and reads back every entry
      of the freshly written archive (new `archive::verify_archive`,
      counting real files, not the directory-entry `append_dir_all`
      always adds even for an empty source) before removing anything, so
      a corrupt or truncated archive is caught while the original still
      exists rather than only discovered once it's already gone.

      New `src/archive.rs` (`archive_vault_dir`, `verify_archive`,
      `unarchive_vault_dir`) does the pure filesystem work; `VaultEntry`
      gained `archived: Option<PathBuf>` (the archive file's location
      when archived — `path` stays what the *live* directory would be,
      so unarchiving knows where to restore to) and `Config` gained
      `archive_vault`/`unarchive_vault` for the registry bookkeeping
      alone — the "is this vault currently mounted" precondition is
      deliberately checked by `main.rs`'s orchestration *before* the
      (potentially slow) compression work runs, not inside `Config`
      after the fact. `vault archive` refuses on a mounted vault
      (archiving something meant to still be live would pull the rug out
      from under it — unmount first) or an already-archived one; `vault
      unarchive` refuses if the destination directory already exists and
      is non-empty, same don't-silently-clobber instinct as `import`.
      Neither auto-mounts/unmounts beyond what archiving/unarchiving
      itself requires — `vault mount` after `unarchive` is a separate,
      explicit step, matching every other `vault ...` subcommand's
      "one command, one effect" shape. `App::new()`'s `unmounted_vaults`
      populator (which drives `TreeRow::UnmountedVault`, see "Multiple
      vaults" above) now excludes archived entries — their placeholder
      row would tell you to `mycora vault mount <name>`, which is wrong
      for something with nothing left at `path` to mount, until archived
      vaults get their own row treatment (still open, see below).

      3 new tests in `archive.rs` (round trip preserves file content
      including a nested subdirectory; an archive with zero files is
      rejected; a non-gzip file is rejected) and 7 in `config.rs`
      (archived path + forced-unmounted set correctly, refuses double-
      archiving, `unarchive` clears the field and refuses on a
      not-archived vault, not-found errors for both). Manually verified
      end to end against a hand-built two-vault registry: `vault archive`
      refused on the mounted vault and (after) on the already-archived
      one; the archived vault's directory was gone and a valid
      `old-notes.tar.gz` existed after a successful archive; `vault list`
      showed `[archived]`; `vault unarchive` restored both notes'
      content byte-for-byte, removed the archive file, and left the
      entry unmounted; `vault mount` + `mycora reindex` afterward loaded
      the restored vault's 2 notes correctly, confirming the round trip
      didn't just look right on disk but was actually usable again.

      **Still open, not built yet**: both archived and (any future)
      differently-treated vault states would want the same tree
      treatment unmounted vaults already got — their own placeholder
      row, and a way to declutter the tree once there are enough of
      them. `:config archive show/hide` (and, before lock was dropped,
      a `:config lock show/hide` alongside it) as a TUI-side display
      toggle was floated for that and confirmed with the user via
      `AskUserQuestion` to mean exactly "show/hide the placeholder
      rows," not something else — but no `:config` command namespace
      exists at all yet, and an archived vault has no placeholder row of
      its own to toggle yet either (see the `unmounted_vaults` exclusion
      above), so this is sequenced as explicit next work, not designed
      further than the confirmed intent.
