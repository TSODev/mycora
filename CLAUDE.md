# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Mycora is a terminal application (Rust, `ratatui` + `crossterm`) for
hierarchical, Markdown-backed note-taking: every note has exactly one parent
(a strict tree, navigated vim-style), plus a "mycelial" cross-link layer on
top (`[[wikilink]]`-style references, independent of tree position, with a
backlinks panel (`b`) and its mirror, an outgoing-links jump (`f`), plus
cross-vault resolution and autocompletion while typing — see `link.rs`'s
`unclosed_wikilink_start`). Both halves are implemented and
shipped, along with SQLite FTS5 search (ranked, with snippets and tag/date/
branch facets), multi-vault mounting (a registry of vaults, only one of
which is editable at a time), a resizable three-pane layout (tree + a
Markdown-rendered body preview + backlinks), a `:` command palette,
session persistence, and a multilingual interface (English/French/
Spanish/German, `config.toml`'s `language` key or `:lang` to switch
live — see `lang.rs`). v0.1 through v0.9 are functionally complete except
one deliberately deferred item — arbitrary configurable keybindings —
see ROADMAP.md for the full staged plan and the reasoning behind every
non-obvious decision along the way.
`examples/showcase-vault/` is a real, committed Mycora vault documenting
Mycora itself (philosophy, interface, features, design decisions, as
interlinked notes) — a good first thing to open when getting oriented, and
a working example of the on-disk file format. Published on crates.io as
`mycora`.

## Commands

```sh
cargo build              # debug build
cargo run                # run the TUI against the configured vault
cargo test                # all unit tests (198 tests, all in-crate, no external deps)
cargo test <substring>    # e.g. `cargo test deep_copy` — matches by test/module name
cargo test -p mycora vault::tests::save_then_load_round_trips_a_note  # single test
cargo clippy
cargo run --example generate-test-vault [output_dir] [leaf_note_count]
mycora reindex [--watch]  # CLI subcommand, not a cargo command — rebuilds
                           # the SQLite index for every mounted vault; --watch
                           # keeps running and reindexes on file changes
```

- Edition 2024, no `rust-toolchain` file pinned; current toolchain in this
  environment is 1.97. The code uses `let`-chains (`if let ... && let ... {}`,
  see `vault.rs`/`app.rs`) which need a recent-enough edition 2024 compiler —
  if a build fails with a parse error on those `&&`-chained `let`s, suspect
  an old toolchain first.
- No `rustfmt.toml`/`clippy.toml`, no CI workflow (`.github/workflows` does
  not exist), no Cursor/Copilot instruction files in this repo.
- **Tests are all `#[cfg(test)] mod tests` unit tests**, spread across
  `tree.rs`, `vault.rs`, `config.rs`, `index.rs`, `link.rs`, `lang.rs`,
  `markdown.rs`, and `session.rs` — there is no `tests/` integration
  directory. None of
  them need an external service, network, or env var: `vault.rs`/`index.rs`
  tests build a scratch directory under `std::env::temp_dir()` per test
  (unique via a fresh UUID) and clean it up at the end. The only place
  `std::env::var` matters at all is `Config::load()` reading `HOME` at
  runtime (not in tests). No test is `#[ignore]`d or skipped based on
  environment.

## Architecture

Data flow: `main.rs` parses CLI args (`clap`) — either a `reindex [--watch]`
subcommand (`perform_reindex`, exits without touching the terminal) or, with
no subcommand, installs a panic hook, builds an `App` (which loads `Config`,
mounts every vault marked `mounted`, opens the SQLite `Index`, and restores
the last `Session`), enters raw/alternate-screen mode, and loops `ui::draw` +
`event::poll_and_handle` until `app.should_quit`, saving the session on the
way out regardless of whether the loop ended via `q`/`q` or `Ctrl+C`. The
crate is split `lib.rs` + `main.rs` specifically so
`examples/generate-test-vault.rs` can depend on `mycora::vault`/`note`
directly and guarantee its synthetic output matches the real on-disk format.

- **`tree.rs` — `Tree`**: the UI- and disk-agnostic in-memory model.
  `HashMap<NoteId, Note>` plus a `roots: Vec<NoteId>`; every `Note` also
  carries its own `children: Vec<NoteId>` and `order: i64`. Owns all
  structural operations — `create_note`, `move_note` (reparent, with
  `is_descendant` cycle detection — walks *up* from the candidate new
  parent, O(depth)), `move_up`/`move_down` (sibling reorder), `deep_copy`
  (recursive, fresh ids/timestamps for every node), `delete_subtree`
  (returns the removed `(NoteId, Note)` pairs depth-first so a caller can
  restore or persist the removal). `rebuild_hierarchy` is a separate pass
  used only after bulk-loading from disk via `insert_loaded`: it derives
  `roots`/`children` from each note's `parent` field, sorts by `order`, and
  demotes-to-root any note whose parent doesn't resolve (returning those ids
  as "orphaned" for the caller to warn about).
- **`note.rs` — `Note`/`NoteId`**: `NoteId` wraps a `Uuid` v4, generated
  once at creation and persisted in frontmatter — stable across renames and
  moves, unlike a path- or content-derived id.
- **`vault.rs` — `Vault`**: the sole owner of the on-disk note↔path mapping
  and the only thing that touches the filesystem for note data. Markdown is
  the source of truth; the in-memory `Tree` is fully derived from it and
  disposable. File format: `---`-delimited YAML frontmatter (`id`, `parent`,
  `order`, `tags`, `created`, `updated`) followed by `# Title` + body.
  `load()` walks the vault directory, skips malformed files, reassigns
  duplicate ids, and (via `Tree::rebuild_hierarchy`) self-heals orphaned
  parents — every anomaly becomes a `String` warning rather than a crash or
  silent data loss, and self-healing is written back to disk immediately so
  the same warning doesn't repeat on the next load. Writes are atomic
  (temp file + `rename`). `trash_note` moves a file to `<vault>/.trash/`
  instead of deleting it; trash is never auto-scanned or auto-emptied.
  `save_note` returns `Result<bool>` (renamed-the-file-or-not): a
  filename is allocated (`slugify` + `unique_path`) once on a note's
  first save, and every later save compares that file's stem against a
  fresh `slugify(&note.title)`, `fs::rename`-ing to a freshly
  disambiguated path when they differ — the file only ever drifts from
  the title between the note's creation (title still "New note") and
  its first real rename, not indefinitely. `mycora vault
  sync-filenames <name>` (`main.rs`) is the retroactive fixer for notes
  that already drifted before this existed — just every note re-saved
  through the same path.
- **`config.rs` — `Config`**: reads `<config dir>/mycora/config.toml`
  (`~/.config/mycora/config.toml` on Linux, `%APPDATA%\mycora\config.toml`
  on Windows). Holds a
  registry of named vaults (`VaultEntry { name, path, mounted }`);
  `mounted_vaults()` filters to what should load at startup, `active_vault()`
  picks the entry named `"default"` (or the first mounted one if none is) as
  the sole *editable* vault — every other mounted vault is read-only in the
  TUI (see `app.rs`'s `ReadOnlyVault`). A legacy single `vault_path` key is
  still honored as a fallback when `vaults` is empty, so a pre-registry
  config keeps working unchanged. Also holds `language: Lang` (from an
  optional `language = "fr"` key, see `lang.rs`) — an unrecognized code
  fails `load()` loudly rather than silently defaulting to English.
  Config/session/index paths and the default vault's location are all
  resolved via the `dirs` crate (`config_dir()`/`data_dir()`/`home_dir()`)
  rather than a literal `$HOME` read, so they land in the right
  platform-native location on Linux, macOS, and Windows alike (see
  `INSTALL-WINDOWS.md`). Windows behavior is reasoned through, not yet
  confirmed on a real Windows machine — `examples/showcase-*` stays
  Windows-silent until that's verified, deliberately, rather than
  documenting a platform nobody's actually run this on yet.
- **`lang.rs` — `Lang`**: the TUI's interface language (`En`/`Fr`/`Es`/
  `De`, English default) — every label, hint, prompt, and status message
  in `app.rs`/`ui.rs` routes through a `Lang` method (`unknown_command`,
  `mode_line`, `command_reference`, ...) rather than a literal string.
  Every language is embedded as compile-checked `format!` calls, not
  external language files — a missing translation is a compile error
  (the `match self { Lang::En => ..., ... }` exhaustiveness check
  refuses to build otherwise), and adding a language is mechanical
  rather than risky. Keybindings and command syntax (`:tags limit`,
  `show`/`hide`, ...) never translate, same as vim's `:w` — only
  `command_reference`'s *descriptions* differ per language, its
  `(syntax, ...)` halves are asserted identical in a unit test.
  Spanish/German are machine-translated and flagged (in this doc
  comment and USAGE.md) as not yet reviewed by a native speaker, unlike
  the reviewed English/French pair. `:lang <en|fr|es|de>` (`app.rs`'s
  `command_lang`) switches `App::lang` live — every string re-reads it
  on each of `ui.rs`'s ~10fps redraws, so reassigning the field *is*
  the refresh, no separate mechanism needed — and persists the choice
  via `Config::set_language` (same parse-and-rewrite plumbing as
  `add_vault`). CLI output stays English for now; this is TUI-only.
- **`index.rs` — `Index`**: the disposable SQLite index behind search, tag
  filtering, and links (`<data dir>/mycora/index.sqlite3` —
  `~/.local/share` on Linux, `%APPDATA%` on Windows, via `dirs`;
  `rusqlite` `bundled`, no system libsqlite3). Schema: `notes`, `tree_edges`, `tags`,
  `notes_fts` (FTS5 over title/body/tags), and `links`
  (`source_vault`/`source`/`target_vault`/`target` — a single `vault_id`
  column can't express an edge whose two ends live in different vaults).
  `reindex_mounted(&[(vault_id, tree, vault)])` is the real batch API (two
  internal phases, `write_notes` then `write_links`, since link resolution
  needs every vault's notes already written before any of them can be
  looked up); single-vault `reindex` is a one-entry convenience wrapper
  around it. A wikilink title matching more than one note fans out to a
  link per match (deliberate — see ROADMAP.md's "Fan-out ambiguous
  wikilinks" design note — not an error); a title matching nothing becomes
  a `BrokenLink` in the returned `ReindexReport` rather than being silently
  dropped. `search`/`search_faceted` rank via FTS5's built-in BM25 `rank`;
  `search`'s `snippet` comes from FTS5's own `snippet()`, with each matched
  term wrapped in `\u{1}`/`\u{2}` sentinels for `ui.rs` to style rather than
  visible markup. An old on-disk schema shape (e.g. `links` before the
  cross-vault column split) is detected and dropped/recreated on open
  rather than migrated — "the index is disposable" extends to its own
  schema, not just its data. `open()` sets WAL journal mode and a 5s
  `busy_timeout` unconditionally (both core `rusqlite` methods, no
  extra Cargo feature) — readers don't block behind an in-progress
  writer, and a second process racing a reindex waits and retries
  instead of an instant "database is locked". Not the same thing as
  concurrent-write *safety*: two processes can still each believe they
  won a write to the same row; see ROADMAP.md's "Concurrent-write
  safety" entry for the (still open) bigger picture and the two other
  options weighed against this one.
- **`link.rs`**: `extract_wikilink_titles` is a hand-rolled `[[title]]`
  bracket scanner, no `regex` dependency and no `[[Title|alias]]` syntax.
  It's naive: once it sees an unclosed `[[`, it scans forward to the *next*
  `]]` anywhere later in the body as that link's title, even across
  unrelated sentences — don't write a bare illustrative `[[` in a note body
  (e.g. to describe the syntax itself) without a real matching `]]` right
  next to it, or a later, unrelated `]]` will get swallowed into a bogus
  "broken link". `unclosed_wikilink_start(line, cursor_col)` is the
  separate, single-line-scoped scanner backing the body editor's
  autocomplete popup (`App::refresh_link_autocomplete` in `app.rs`) —
  character-indexed to match `ratatui-textarea`'s own cursor addressing,
  not byte offsets.
- **`markdown.rs`**: `render(&str, width: u16) -> Vec<Line>` walks
  `pulldown-cmark`'s event stream (`Parser::new_ext(source,
  Options::ENABLE_TABLES)` — the crate's `default-features = false` only
  trims unrelated Cargo features, table parsing is a runtime `Options`
  flag and unaffected) and builds styled `ratatui::text::Line`s directly
  (a small hand-rolled `Renderer` with a style stack, not a dedicated
  markdown-widget crate). Used by `ui.rs`'s body preview pane (called
  with `chunks[0].width`, the inner pane width post-border/padding);
  read-only and not interactive — links and `[[wikilinks]]` render as
  plain text. Tables are the one block that can't stream straight to
  `self.lines` like every other event: column widths depend on every
  row, so cells are buffered (`table_rows: Vec<Vec<Vec<Span>>>`) until
  `TagEnd::Table`, then rendered as a bordered grid (box-drawing
  characters, dimmed) with a bold header row and per-column alignment
  honoring GFM's `| :--- | ---: | :---: |` markers. `width` exists
  *only* for this: `ui.rs` applies ratatui's `Wrap` widget on top of
  every rendered `Line` for ordinary prose reflow, but `Wrap` breaks
  lines at arbitrary points to fit the pane — fatal for a box-drawn
  table, since it slices straight through the borders. So
  `allocate_column_widths` shrinks columns to fit `width` first (every
  column's floor-divided share of the budget, proportional to its ideal
  content width) and `wrap_cell` greedily word-wraps each cell into that
  column, hard-breaking at the character level as a last resort for a
  single word too long to fit its column at all (e.g. a URL) — so every
  line the table emits is *exactly* `width` columns wide and `Wrap`
  never has anything left to do to it. Everything else (prose,
  headings, lists, code) skips this entirely and still relies on
  `ui.rs`'s `Wrap` for reflow, same as before tables existed.
- **`session.rs` — `Session`**: reads/writes
  `<data dir>/mycora/session.toml`, keyed by vault name
  (`selected`/`expanded` per vault, so switching which vault is `"default"`
  doesn't clobber another vault's remembered position). Saved once at
  shutdown (`App::save_session`, called from `main.rs` right after `run()`
  returns), not write-through — this is ephemeral navigation state, not
  user content. Restored ids that no longer resolve (note deleted, vault
  changed) are dropped rather than kept dangling.
- **`app.rs` — `App`**: UI-facing state machine wrapping the active
  `Tree` + `Vault`, every other mounted-but-read-only vault
  (`ReadOnlyVault { id, tree, vault }`), and the shared `Index`. `Mode` is
  `Normal | Insert | ConfirmDelete | Search | Backlinks | EditBody |
  Command | TagResults | TagList | Links | Help` — dispatch lives in `event.rs`, rendering in
  `ui.rs` (see those files' notes on which modes are full-pane overlays vs.
  status-bar-only prompts vs. in-place pane focus). Every mutating method
  (`create_child`, `commit_rename`, `confirm_delete`, `indent_selected`,
  `reorder_up`, ...) follows the same pattern: mutate `self.tree`, call
  `self.persist(id)` to write through to the vault immediately (no explicit
  save step anywhere), then `self.record(UndoAction::…)`. **Undo/redo is
  built on inverses computed against the *live* tree at apply time**, not
  snapshots frozen when the action was recorded — `apply_undo_action`
  reads the note's *current* state before mutating it and pushes that as
  the entry on the opposite stack. This is what keeps a chain of undo/redo
  correct even when other edits happened in between; don't "simplify" this
  into replaying stored snapshots without preserving that property.
  `visible_rows()` (depth-first, respecting `expanded: HashSet<NoteId>`)
  is recomputed on every call rather than cached — acceptable at current
  scale per ROADMAP's v0.1 note, revisit if it shows up in profiling. It
  returns `TreeRow::Note`/`TreeRow::VaultSeparator` spanning the active
  vault *and* every read-only mounted one (real navigation, not
  roots-only) — `resolve(id) -> Option<(&Tree, &str)>` is the backbone
  every cross-vault read accessor (`live_backlinks`, `selected_note`,
  `breadcrumb_titles`, ...) uses to find which tree an id actually
  belongs to, and `require_editable(id)` is the guard every mutating
  method checks first, refusing with `last_error` rather than silently
  no-oping or acting on the wrong vault if `id` isn't in the active tree.
  `Lang::command_reference()` (`&[(syntax, description)]`, in `lang.rs`)
  is the single source both `execute_command`'s dispatch and `ui.rs`'s
  command-palette help popup read from — keep the two in sync by hand
  if the command set grows.
- **`event.rs`**: crossterm key polling (100ms), dispatches on `app.mode`.
  `Ctrl+C` is handled unconditionally before mode dispatch (raw mode
  disables SIGINT generation, so this is the only way to get an emergency
  quit) — it bypasses both the delete-confirmation prompt and the `q`/`q`
  double-press-to-quit convention (`request_quit`/`confirm_quit`), which
  exists specifically because a single stray `q` used to close the app with
  no way back. In Normal mode: `[`/`]`/`{`/`}` resize the tree/backlinks
  panes (always active, no dedicated resize mode), `b` toggles keyboard
  focus onto the backlinks pane in place (not a separate overlay), `:`
  opens the command palette (`Mode::Command`).
- **`ui.rs`**: pure rendering from `App` state. Two vertical chunks: the
  main area, and a `Length(2)` status band (`Color::Indexed(236)` bg,
  harmonized with Terapi/jsoned's convention) split into a breadcrumb row
  and a hint/prompt row. The main area is a resizable three-pane split
  (tree/blue border, Markdown-rendered body preview/magenta border,
  backlinks) for every mode except the full-pane overlays (`Search`,
  `EditBody`, `TagResults`), which take over the whole area instead.
  `Mode::Command` additionally overlays a small `Clear`-first help popup
  (`draw_command_help`, anchored bottom-center via `popup_rect`) listing
  `Lang::command_reference()` (in `app.lang`'s language) for as long as
  the `:` prompt is open. Every string rendered here — titles, hints,
  prompts, markers — reads `app.lang` rather than a literal, which is
  also the entire mechanism behind `:lang` switching live (see
  `lang.rs`): nothing here caches or needs invalidating. Every color is
  a named ANSI color, not RGB/indexed, so light/dark theming comes from
  whatever the terminal itself is configured with — no in-app theme
  switch. No state lives here.
  Normal mode's hint row (`draw_hint_row`) deliberately shows only a
  short, curated subset of keys (`Lang::mode_line`'s `Normal` arm) — the
  full set once ran to 233 characters, wider than any real terminal;
  `?` (`Mode::Help`, `draw_help`) is a full-pane overlay over
  `Lang::help_reference()` for everything else, dismissed by any
  keypress rather than requiring `Esc` specifically (there's no
  selection to navigate, just a static list). `draw_breadcrumb` adds a
  third, centered segment — the selected note's `updated` timestamp
  (`format_last_modified`, UTC, plain `OffsetDateTime` field access
  rather than `time::format_description` to avoid needing the
  unenabled `macros` Cargo feature) — via `Constraint::Fill(1)` on both
  sides of the fixed-width label, *not* the breadcrumb's own existing
  `Min(0)` chunk, so it centers on the *whole* row regardless of
  breadcrumb length; hidden below `MIN_BREADCRUMB_RESERVE` rather than
  ever being squeezed. The two `Fill(1)` sides split *remaining* space
  *equally* — the show/hide guard has to clear each side's own minimum
  individually (`2 * max(reserve, marker_width)`), not just their sum.

## Known pitfall (from CHANGELOG)

A panic while the TUI was running used to leave the terminal broken (raw
mode + alternate screen never torn down on a panic path) until the user ran
`reset`/`stty sane`. Fixed by installing a panic hook in `main()` that
restores the terminal before the default panic report prints — if you touch
terminal setup/teardown in `main.rs`, keep that hook intact and keep it
installed *before* `enable_raw_mode()`/`EnterAlternateScreen`.
