# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Mycora is a terminal application (Rust, `ratatui` + `crossterm`) for
hierarchical, Markdown-backed note-taking: every note has exactly one parent
(a strict tree, navigated vim-style), plus a "mycelial" cross-link layer on
top (`[[wikilink]]`-style references, independent of tree position, with a
backlinks panel and cross-vault resolution). Both halves are implemented and
shipped, along with SQLite FTS5 search (ranked, with snippets and tag/date/
branch facets), multi-vault mounting (a registry of vaults, only one of
which is editable at a time), a resizable three-pane layout (tree + a
Markdown-rendered body preview + backlinks), a `:` command palette, and
session persistence. v0.1 through v0.7 are functionally complete except two
deliberately deferred items — link autocompletion while typing `[[`, and
arbitrary configurable keybindings — see ROADMAP.md for the full staged
plan and the reasoning behind every non-obvious decision along the way.
`examples/showcase-vault/` is a real, committed Mycora vault documenting
Mycora itself (philosophy, interface, features, design decisions, as
interlinked notes) — a good first thing to open when getting oriented, and
a working example of the on-disk file format. Published on crates.io as
`mycora`.

## Commands

```sh
cargo build              # debug build
cargo run                # run the TUI against the configured vault
cargo test                # all unit tests (83 tests, all in-crate, no external deps)
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
  `tree.rs`, `vault.rs`, `config.rs`, `index.rs`, `link.rs`, `markdown.rs`,
  and `session.rs` — there is no `tests/` integration directory. None of
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
- **`config.rs` — `Config`**: reads `~/.config/mycora/config.toml`. Holds a
  registry of named vaults (`VaultEntry { name, path, mounted }`);
  `mounted_vaults()` filters to what should load at startup, `active_vault()`
  picks the entry named `"default"` (or the first mounted one if none is) as
  the sole *editable* vault — every other mounted vault is read-only in the
  TUI (see `app.rs`'s `ReadOnlyVault`). A legacy single `vault_path` key is
  still honored as a fallback when `vaults` is empty, so a pre-registry
  config keeps working unchanged. Requires `HOME` to be set.
- **`index.rs` — `Index`**: the disposable SQLite index behind search, tag
  filtering, and links (`~/.local/share/mycora/index.sqlite3`, `rusqlite`
  `bundled`, no system libsqlite3). Schema: `notes`, `tree_edges`, `tags`,
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
  schema, not just its data.
- **`link.rs`**: `extract_wikilink_titles` is a hand-rolled `[[title]]`
  bracket scanner, no `regex` dependency and no `[[Title|alias]]` syntax.
  It's naive: once it sees an unclosed `[[`, it scans forward to the *next*
  `]]` anywhere later in the body as that link's title, even across
  unrelated sentences — don't write a bare illustrative `[[` in a note body
  (e.g. to describe the syntax itself) without a real matching `]]` right
  next to it, or a later, unrelated `]]` will get swallowed into a bogus
  "broken link".
- **`markdown.rs`**: `render(&str) -> Vec<Line>` walks `pulldown-cmark`'s
  event stream and builds styled `ratatui::text::Line`s directly (a small
  hand-rolled `Renderer` with a style stack, not a dedicated
  markdown-widget crate). Used by `ui.rs`'s body preview pane; read-only
  and not interactive — links and `[[wikilinks]]` render as plain text.
- **`session.rs` — `Session`**: reads/writes
  `~/.local/share/mycora/session.toml`, keyed by vault name
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
  Command | TagResults` — dispatch lives in `event.rs`, rendering in
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
  `visible_notes()` (depth-first, respecting `expanded: HashSet<NoteId>`)
  is recomputed on every call rather than cached — acceptable at current
  scale per ROADMAP's v0.1 note, revisit if it shows up in profiling.
  `COMMAND_REFERENCE` (`&[(syntax, description)]`) is the single source
  both `execute_command`'s dispatch and `ui.rs`'s command-palette help
  popup read from — keep the two in sync by hand if the command set grows.
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
  `COMMAND_REFERENCE` for as long as the `:` prompt is open. Every color is
  a named ANSI color, not RGB/indexed, so light/dark theming comes from
  whatever the terminal itself is configured with — no in-app theme
  switch. No state lives here.

## Known pitfall (from CHANGELOG)

A panic while the TUI was running used to leave the terminal broken (raw
mode + alternate screen never torn down on a panic path) until the user ran
`reset`/`stty sane`. Fixed by installing a panic hook in `main()` that
restores the terminal before the default panic report prints — if you touch
terminal setup/teardown in `main.rs`, keep that hook intact and keep it
installed *before* `enable_raw_mode()`/`EnterAlternateScreen`.
