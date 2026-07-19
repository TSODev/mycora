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
cargo test                # all unit tests (232 tests, all in-crate, no external deps)
cargo test <substring>    # e.g. `cargo test deep_copy` — matches by test/module name
cargo test -p mycora vault::tests::save_then_load_round_trips_a_note  # single test
cargo clippy
cargo run --example generate-test-vault [output_dir] [leaf_note_count]
cargo run --release --example benchmark -- 100 1000 5000 10000
                           # timed load/reindex/search/visible_rows passes
                           # at each given vault size — see BENCHMARK.md
mycora reindex [--watch]  # CLI subcommand, not a cargo command — rebuilds
                           # the SQLite index for every mounted vault; --watch
                           # keeps running and reindexes on file changes
mycora repair [--apply] [--create-stubs] [--vault <name>]
                           # CLI subcommand — reports (and optionally fixes)
                           # broken [[wikilink]]s; see repair.rs below
```

- Edition 2024, no `rust-toolchain` file pinned; current toolchain in this
  environment is 1.97. The code uses `let`-chains (`if let ... && let ... {}`,
  see `vault.rs`/`app.rs`) which need a recent-enough edition 2024 compiler —
  if a build fails with a parse error on those `&&`-chained `let`s, suspect
  an old toolchain first.
- No `rustfmt.toml`/`clippy.toml`, no Cursor/Copilot instruction files in
  this repo. `.github/workflows/windows-release.yml` is the one CI
  workflow that does exist (see ROADMAP.md's Windows support entry) —
  not a test/lint gate on every push, just a release-time job: on every
  `v*` tag push (which `PUBLISH.md`'s own release flow ends with) it
  builds a native Windows binary on `windows-latest` (not cross-compiled
  from Linux, specifically so `rusqlite`'s `bundled` feature has a real
  MSVC toolchain to compile SQLite's C source against) and attaches it
  to the matching GitHub Release.
- **Tests are all `#[cfg(test)] mod tests` unit tests**, spread across
  `tree.rs`, `vault.rs`, `config.rs`, `index.rs`, `link.rs`, `lang.rs`,
  `markdown.rs`, `outline.rs`, `import.rs`, `repair.rs`, `session.rs`,
  `app.rs`, `clipboard.rs`, `archive.rs`, and `export.rs` — there is no
  `tests/` integration directory. None of
  them need an external service, network, or env var: `vault.rs`/`index.rs`
  tests build a scratch directory under `std::env::temp_dir()` per test
  (unique via a fresh UUID) and clean it up at the end. The only place
  `std::env::var` matters at all is `Config::load()` reading `HOME` at
  runtime (not in tests). No test is `#[ignore]`d or skipped based on
  environment. `app.rs`'s own tests are the newest and sparsest of the
  bunch: `App::new()` isn't test-friendly on its own (it always reads
  the real user config/session/index paths via `Config::load`/
  `Session::default_path`/`Index::default_path`), so a `#[cfg(test)]`-
  only `App::new_for_test(tree, vault, index, vault_id)` builds one from
  already-in-memory pieces instead, the same scratch-`Tree`/`Vault`/
  `Index` construction `vault.rs`/`index.rs`'s own tests already use,
  just assembled into a full `App` rather than exercised piece by piece.

## Architecture

Data flow: `main.rs` parses CLI args (`clap`) — either a `reindex [--watch]`
subcommand (`perform_reindex`, exits without touching the terminal), a
`repair [--apply] [--create-stubs] [--vault <name>]` subcommand
(`perform_repair`, shares `perform_reindex`'s `load_and_reindex_mounted`
loader — see `repair.rs` below), or, with
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
- **`archive.rs`**: `archive_vault_dir`/`unarchive_vault_dir` gzip-tar a
  vault directory to a single `.tar.gz` and back (`tar` + `flate2`, paths
  inside relative to the vault dir itself, not prefixed with its own
  directory name, so the round trip works regardless of what either end
  is named) — backing `mycora vault archive <name> [output]`/`vault
  unarchive <name>` in `main.rs`, the "make an unmounted vault's
  directory stop existing on disk without deleting it" pair
  (`TreeRow::ArchivedVault` in `app.rs` is the tree pane's placeholder
  for one). Archiving refuses a still-mounted vault (unmount first) and
  calls `verify_archive` right after writing — reads every entry's
  header (no extraction) and counts regular files, failing loudly on a
  corrupt/truncated archive or an accidentally-empty one — *before*
  `main.rs`'s caller removes the original directory, so a bad archive is
  never the last copy of the vault's notes.
- **`import.rs`**: reads *foreign* Markdown into a `Tree`, mirroring
  `Vault::load`'s `(Tree, warnings)` shape for sources that aren't
  Mycora's own format. `import_obsidian_vault(dir)` (bulk, `mycora vault
  import` CLI subcommand in `main.rs`) walks a directory tree — a
  subdirectory becomes a parent note (reusing a same-named sibling `.md`
  as that note's own content if one exists, else a synthesized empty
  one), `.obsidian/` and non-`.md` files are skipped. `parse_foreign_note
  (path, raw) -> (title, body, tags, warning)` is the shared per-file
  parser behind *both* that bulk import and `App::command_import`
  (`:import <path>`, a single file into the active vault as a child of
  the selected note) — title from the filename (unlike
  `vault.rs::split_title`'s Mycora-native "first `# Heading` is the
  title" rule), tags from optional YAML frontmatter (best-effort:
  unparseable frontmatter becomes `warning`, not a failed import), and
  every `[[Title|Alias]]`/`[[Title#Heading]]` rewritten to a plain
  `[[Title]]` since `link.rs`'s scanner only understands that bare form.
  One parser, two call sites, so a file means the same thing regardless
  of which door it came in through.
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
  "broken link". One exception it *does* know about: `code_ranges(body)`
  runs a `pulldown-cmark` parse first (same `Options::ENABLE_TABLES` as
  `markdown.rs`/`outline.rs`) to collect the byte ranges of fenced code
  blocks and inline code spans, and both `extract_wikilink_titles` and
  `rewrite_wikilink_title` skip any `[[...]]` whose start falls inside
  one — otherwise TOML's array-of-tables syntax (`[[campaign.steps]]`)
  or similar shown as a code example inside a note reads as a broken
  wikilink on every reindex. `unclosed_wikilink_start(line, cursor_col)`
  is the separate, single-line-scoped scanner backing the body editor's
  autocomplete popup (`App::refresh_link_autocomplete` in `app.rs`) —
  character-indexed to match `ratatui-textarea`'s own cursor addressing,
  not byte offsets, and *not* code-block-aware (it has no whole-body
  Markdown context to parse mid-keystroke). `rewrite_wikilink_title(body,
  old_title, new_title)` is the same bracket-scanning idiom used to
  *rewrite* rather than extract — every `[[old_title]]` occurrence
  outside a code range becomes `[[new_title]]`, backing `mycora repair
  --apply`'s retargeting (see `repair.rs` below).
- **`outline.rs`**: heading/section geometry for the `t` table-of-contents
  overlay — a separate lexical concern from `markdown.rs`'s styling, so
  it's its own small module (same shape as `link.rs`) rather than bolted
  onto `Renderer`. `headings(source) -> Vec<HeadingRef>` walks
  `Parser::new_ext(source, Options::ENABLE_TABLES).into_offset_iter()`
  (the same options as `markdown::render`, so a `#` inside a table cell
  or fenced code block is never mistaken for a heading here either) and
  records each heading's level, trimmed title, and *byte* start/end
  offsets in the source — not rendered-line indices, which stay
  width-dependent (see below). `section_range`/`extract_section` turn a
  heading index into the byte range it owns (up to the next heading at
  the same or a shallower level, or end of body) and slice it out —
  extraction is inherently non-recursive since it's one contiguous byte
  range; a deeper sub-heading inside is just part of that slice, never a
  boundary. `scroll_offset_for(source, byte_offset)` (used by
  `App::confirm_toc` to jump to a heading, and — despite the name and
  module, since it isn't actually heading-specific — reused verbatim by
  `App::confirm_broken_wikilinks` to land near a broken wikilink's own
  position) sidesteps the fact that `App` never knows the live
  body-preview pane width (`ui.rs` is pure rendering — see its own note
  below) by re-calling
  `markdown::render` on `source[..heading_start]` at a fixed constant
  width and counting the lines produced — exact for every block type
  except tables (width-sensitive), gracefully approximate otherwise, the
  same accepted imprecision as `App::scroll_body_down`'s own doc
  comment.
- **`export.rs`**: `flatten_subtree(tree, root)` walks a subtree
  depth-first into one Markdown document (each note's title becomes a
  heading at a depth-matching level, its own ATX headings shifted
  deeper to nest under it rather than compete with it) — shared by
  `:export`/`mycora export` (see `showcase-vault`'s
  `pdf-export-renders-through-a-pure-rust-crate.md` for the original
  `markdown2pdf` choice). `write_output(content, path)` is the single
  place both call to actually write it: a `.pdf` path renders through
  `markdown2pdf::parse_into_file`, anything else is written verbatim as
  Markdown. Passes a `FontConfig` pointing at an embedded DejaVu
  Sans/Sans Mono (`assets/fonts/`, Bitstream Vera License, ~1.1MB in
  the binary) rather than leaving it `None` — `markdown2pdf`'s own
  default with no font configured falls back to the 14 standard PDF
  fonts, which (per its own `to_win1252` doc comment) only transliterate
  a curated set of punctuation and replace *everything* else —
  accented Latin included — with a literal `?`; DejaVu covers Latin
  Extended/Greek/Cyrillic (not CJK/emoji, which would need a much
  bigger font — out of scope, see the `Unreleased` `CHANGELOG.md`
  entry). Embedded via `include_bytes!` rather than
  `FontSource::System` specifically to keep PDF export self-contained,
  same reasoning as choosing `markdown2pdf` over shelling out to
  `pandoc`/`wkhtmltopdf` in the first place. Bold text (every heading,
  since `flatten_subtree` makes note titles into headings) renders in
  the same regular weight instead of a true bold face —
  `markdown2pdf` only auto-discovers a bold sibling file next to an
  on-disk font (`FontSource::File`/`System`, by filename convention),
  and an embedded `FontSource::Bytes` has no path for that; falling
  back to the *regular* embedded font rather than a *builtin* bold one
  keeps Unicode correct at the cost of true boldness, which is the
  actual bug this exists to fix. `write_output_embeds_a_unicode_font_
  for_pdf_paths`'s test can't assert the rendered glyphs without a
  PDF-parsing dependency (`markdown2pdf` compresses object streams, so
  even the font dictionary isn't visible to a plain byte search), so it
  compares output size against the same content rendered through the
  crate's own builtin-font path directly — a subsetted embedded font
  adds several KB, a signal that would go quiet if `write_output` ever
  stopped passing a font config through.
- **`clipboard.rs`**: `copy_to_system_clipboard(text)` (backing `Y`) writes
  an OSC 52 escape sequence straight to stdout rather than depending on an
  OS-level clipboard crate (`arboard` and similar need direct X11/Wayland
  access) — works over SSH too, since it's the *client*-side terminal that
  intercepts the sequence, not the remote shell. Includes its own tiny
  base64 encoder (RFC 4648, padded) rather than a dependency for
  something this small and stable; a known-vector test backs it.
  Tmux-aware: `osc52_sequence(text, in_tmux)` — split out as a pure
  function so the exact wrapped byte layout can be asserted in a test
  without a real tmux session — wraps the sequence in tmux's DCS
  passthrough (`ESC P tmux ; ESC <sequence> ESC \`, with the sequence's
  own leading `ESC` doubled, since tmux's DCS parser strips one layer)
  whenever the `TMUX` env var is set, the same detection every other
  OSC 52 tool uses; tmux otherwise swallows an arbitrary escape sequence
  from the program it's running rather than forwarding it to the real
  terminal underneath. `App` can't perform the write itself (it doesn't
  own stdout/the `Terminal`), so `copy_body_to_clipboard` only queues the
  text into `clipboard_copy`; `main.rs`'s `run` loop drains it with
  `take_clipboard_copy` right after each event, same request/consume
  shape as `Ctrl+L`'s `force_redraw`.
- **`repair.rs`**: pure suggestion logic shared by `mycora repair` (CLI)
  and `:brokenlinks` (TUI) — no I/O, no `Tree`/`Vault`, same split as
  `outline.rs` (logic here, orchestration in the caller: `main.rs`'s
  `perform_repair` for the CLI, `app.rs`'s `begin_broken_wikilinks` for
  the TUI — both call the exact same `suggest`, no matching logic
  duplicated between them).
  `suggest(broken_title, candidates) -> Option<Suggestion>` first tries
  a case-insensitive exact match (`Confidence::Certain` — Mycora's own
  title matching is case-sensitive, so this is the single most likely
  real cause of a broken link), then falls back to
  `strsim::jaro_winkler` on lowercased titles (`Confidence::Likely`),
  refusing to guess (`None`) below a 0.85 similarity threshold or
  within 0.05 of the second-best candidate (two similarly-named notes —
  ambiguous, not a confident match). `strsim` was already a transitive
  dependency of `clap_builder` (its own "did you mean" arg-name
  suggestions) before this — promoted to a direct one rather than
  hand-rolling similarity scoring, since it added no new compiled code.
  The actual fix application (rewriting a note's body via
  `link::rewrite_wikilink_title`, or creating a stub note via
  `Tree::create_note`) lives in `main.rs::perform_repair`, not here —
  see the Data flow note above.
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
  `ui.rs`'s `Wrap` for reflow, same as before tables existed. Every
  width/padding calculation in the table path (`cell_width`,
  `allocate_column_widths`, `wrap_cell`) goes through the `unicode-width`
  crate rather than `str::chars().count()` — a `char` is not a terminal
  column: `❌`/`✅` and CJK text are each one `char` but render two
  columns wide, so sizing by char count alone drifted a table's right
  border out of alignment starting on the first double-wide row. This
  is a real (explicit) Cargo dependency even though `ratatui-core`
  already pulls it in transitively — relying on a transitive version
  silently would break the moment ratatui's own dependency changed.
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
  Command | TagResults | TagList | Links | BrokenWikilinks | Toc | Help` — dispatch lives in `event.rs`, rendering in
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
  `UndoAction::Compound(Vec<UndoAction>)` composes several of these into
  one stack entry — `apply_undo_action` recurses into each sub-action
  (still against live state) and collects their inverses, reversed, into
  another `Compound` for the opposite stack. The only current user is
  `extract_toc_selection` (`t`'s TOC overlay, `x` to extract a heading's
  section into a new child note + rewrite the source body with a
  `[[wikilink]]`) — both halves undo/redo as a single `u`/`Ctrl+R`
  instead of two, since each sub-action already independently calls
  `set_selected`. The `Compound` arm also calls `self.reindex_mounted()`
  after applying its sub-actions — deliberately *not* pushed down into
  `extract_toc_selection` alone, since undo/redo re-enter through this
  same arm and need the same fix: extraction is the one action
  guaranteed to add or remove both a `[[wikilink]]` *and* the note it
  resolves to together, so without this, `b` on the note right after
  `x`, `u`, or `Ctrl+R` could show a stale (usually empty) backlinks
  pane until something else happened to reindex — the same staleness
  `begin_links` already avoids by reindexing before showing outgoing
  links. Every other `UndoAction` arm leaves the index alone, same as
  the plain (non-Compound) actions that record them.
  **`nav_history: Vec<NoteId>`** is a separate, much simpler session-only
  stack (no inverses, no redo side) behind `Ctrl+O`
  (`navigate_back`) — `record_nav_jump(id)` pushes the *current*
  selection (not `id`) right before each of the five `confirm_*` jump
  methods (`confirm_search`/`confirm_backlinks`/`confirm_links`/
  `confirm_tag_results`/`confirm_broken_wikilinks`) calls
  `reveal`+`set_selected`, so popping it later returns to where you
  were, browser-back-button style. Deliberately not wired into
  `set_selected` itself or `move_selection` — that would push an entry
  on every single `j`/`k` tree step, turning "walk back through your
  last few jumps" into "walk back through your last few keystrokes."
  `navigate_back` itself never pushes (it would need a mirrored
  "forward" stack to undo cleanly, which nothing asked for yet) — it
  only pops and jumps.
  **`reindex_mounted`** (private) returns `Vec<(String, ReindexReport)>`
  — every mounted vault's name paired with its *full* report, not a
  summed note count — specifically so `begin_broken_wikilinks`
  (`:brokenlinks`) can read `broken_links` out of it; `command_reindex`
  is the one caller that still wants a total, so it sums
  `report.note_count` itself instead. The other four call sites
  (`begin_search`, `begin_links`, `extract_toc_selection`'s reindex, the
  `Compound` arm above) only ever branched on `Err`/`Ok(_)` and needed
  no changes when this widened. `begin_broken_wikilinks` enriches each
  `BrokenLink { source, title }` (bare id + unresolved title, nothing
  else) into a `BrokenWikilinkHit` for `ui.rs` — resolved source title
  and vault via `resolve(broken.source)`, plus a fix suggestion via
  `repair::suggest` (the exact same pure function `mycora repair`
  uses, reused verbatim — no duplicated matching logic between the CLI
  and the TUI). `confirm_broken_wikilinks` does one thing beyond every
  other `confirm_*`: after `reveal`+`set_selected`, it searches the
  now-current note's body for the literal `[[broken title]]` text and,
  if found, sets `body_scroll` via `outline::scroll_offset_for` — which
  isn't actually heading-specific despite living in `outline.rs` and
  being named for that use, it just renders a prefix and counts lines
  for *any* byte offset, so reusing it here needed zero changes to that
  module. Landing on the exact line instead of just the top of the note
  is what makes "`Enter` then `e`" (the whole point of this overlay
  over the CLI's `--apply`) fast on a note longer than a few lines.
  `visible_rows()` (depth-first, respecting `expanded: HashSet<NoteId>`)
  is recomputed on every call rather than cached — acceptable at current
  scale per ROADMAP's v0.1 note, revisit if it shows up in profiling. It
  returns `TreeRow::Note`/`TreeRow::VaultSeparator` spanning the active
  vault *and* every read-only mounted one (real navigation, not
  roots-only) — `resolve(id) -> Option<(&Tree, &str)>` is the backbone
  every cross-vault read accessor (`live_backlinks`, `selected_note`,
  `breadcrumb_titles`, `parent_title_of`, ...) uses to find which tree
  an id actually belongs to — `parent_title_of(id)` specifically backs
  `ui.rs`'s backlinks pane, which appends `" (parent title)"` to each
  entry so two similarly-titled notes (e.g. more than one
  "Introduction") stay distinguishable before you actually jump to
  either — and `require_editable(id)` is the guard every mutating
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
  opens the command palette (`Mode::Command`). `Ctrl+d`/`Ctrl+u`/`Ctrl+r`/
  `Ctrl+o` are each checked early in `handle_normal` before the main
  `match key.code`, same reasoning repeated four times: a plain
  `KeyCode::Char` match can't distinguish `Ctrl+o` from bare `o` (new
  sibling note), so the modifier has to be checked first or the two
  keys would be indistinguishable.
- **`ui.rs`**: pure rendering from `App` state. Two vertical chunks: the
  main area, and a `Length(2)` status band (`Color::Indexed(236)` bg,
  harmonized with Terapi/jsoned's convention) split into a breadcrumb row
  and a hint/prompt row. The main area is a resizable three-pane split
  (tree/blue border, Markdown-rendered body preview/magenta border,
  backlinks) for every mode except the full-pane overlays (`Search`,
  `EditBody`, `TagResults`), which take over the whole area instead.
  `Mode::Command` additionally overlays a small `Clear`-first help popup
  (`draw_command_help`, anchored bottom-center via `popup_rect`, drawn
  only while `App::command_help_open()`) listing `Lang::command_reference()`
  (in `app.lang`'s language) for as long as the `:` prompt is open, with a
  reversed-style cursor (`App::command_help_selected`) on one row —
  `Up`/`Down` (`App::move_command_help_selection`) move it, overwrite
  `command_input` with that entry's syntax each time (leading `:` and any
  `<placeholder>` stripped by `command_help_fill_text`), and arm
  `command_help_navigated`, so arrowing to a command is a shortcut for
  typing it rather than firing it immediately. The next `Enter`
  (`App::execute_command`) checks that flag first: if set, it just hides
  the popup (`command_help_open = false`) and clears the flag rather than
  running the picked syntax as-is — which would fail outright for
  anything still missing a `<placeholder>`'s worth of argument, e.g.
  `:export ` — leaving `command_input` and `Mode::Command` untouched so
  the rest can still be typed; arrowing again (even after a dismiss)
  reopens the popup and re-arms the flag, and a command typed by hand
  without ever touching the list (`command_help_navigated` never set)
  still runs on a single `Enter`, unchanged from before this existed.
  Every string rendered here — titles, hints,
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
