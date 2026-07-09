# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Mycora is a terminal application (Rust, `ratatui` + `crossterm`) for
hierarchical, Markdown-backed note-taking: every note has exactly one parent
(a strict tree, navigated vim-style), plus a separate "mycelial" cross-link
layer planned for later (`[[wikilink]]`-style references, independent of
tree position). The tree is implemented and shipped; cross-links, search
(SQLite FTS5 → tantivy), and a split-pane layout are not — see ROADMAP.md
for the full staged plan (currently through v0.3, "Full tree operations").
Published on crates.io as `mycora`.

## Commands

```sh
cargo build              # debug build
cargo run                # run the TUI against the configured vault
cargo test                # all unit tests (23 tests, all in-crate, no external deps)
cargo test <substring>    # e.g. `cargo test deep_copy` — matches by test/module name
cargo test -p mycora vault::tests::save_then_load_round_trips_a_note  # single test
cargo clippy
cargo run --example generate-test-vault [output_dir] [leaf_note_count]
```

- Edition 2024, no `rust-toolchain` file pinned; current toolchain in this
  environment is 1.97. The code uses `let`-chains (`if let ... && let ... {}`,
  see `vault.rs`/`app.rs`) which need a recent-enough edition 2024 compiler —
  if a build fails with a parse error on those `&&`-chained `let`s, suspect
  an old toolchain first.
- No `rustfmt.toml`/`clippy.toml`, no CI workflow (`.github/workflows` does
  not exist), no Cursor/Copilot instruction files in this repo.
- **Tests are all `#[cfg(test)] mod tests` unit tests inside `src/tree.rs`
  and `src/vault.rs`** — there is no `tests/` integration directory. None of
  them need an external service, network, or env var: `vault.rs` tests build
  a scratch directory under `std::env::temp_dir()` per test (unique via a
  fresh UUID) and clean it up at the end. The only place `std::env::var`
  matters at all is `Config::load()` reading `HOME` at runtime (not in
  tests). No test is `#[ignore]`d or skipped based on environment.

## Architecture

Data flow: `main.rs` installs a panic hook, then builds an `App` (which
loads `Config` and a `Vault`, producing a `Tree` plus load warnings),
enters raw/alternate-screen mode, and loops `ui::draw` + `event::poll_and_handle`
until `app.should_quit`. The crate is split `lib.rs` + `main.rs` specifically
so `examples/generate-test-vault.rs` can depend on `mycora::vault`/`note`
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
- **`app.rs` — `App`**: UI-facing state machine wrapping `Tree` + `Vault`.
  `Mode` is `Normal | Insert | ConfirmDelete`. Every mutating method
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
- **`event.rs`**: crossterm key polling (100ms), dispatches on `app.mode`.
  `Ctrl+C` is handled unconditionally before mode dispatch (raw mode
  disables SIGINT generation, so this is the only way to get an emergency
  quit) — it bypasses both the delete-confirmation prompt and the `q`/`q`
  double-press-to-quit convention (`request_quit`/`confirm_quit`), which
  exists specifically because a single stray `q` used to close the app with
  no way back.
- **`ui.rs`**: pure rendering from `App` state, two vertical chunks (tree
  list + one-line status bar). No state lives here.
- **`config.rs`**: reads `~/.config/mycora/config.toml` (`vault_path`,
  optional, defaults to `~/mycora`). Requires `HOME` to be set.

## Known pitfall (from CHANGELOG)

A panic while the TUI was running used to leave the terminal broken (raw
mode + alternate screen never torn down on a panic path) until the user ran
`reset`/`stty sane`. Fixed by installing a panic hook in `main()` that
restores the terminal before the default panic report prints — if you touch
terminal setup/teardown in `main.rs`, keep that hook intact and keep it
installed *before* `enable_raw_mode()`/`EnterAlternateScreen`.
