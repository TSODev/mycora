# Mycora

**A tree-native, mycelium-linked note-taking TUI, written in Rust.**

Mycora is a terminal application for building and navigating hierarchical
notes — mind-map-style trees — while letting individual notes reference each
other across branches, the way a mycelial network links the root systems of
otherwise separate trees.

> Working and daily-usable. See [Features](#features) below for what's
> built, and [USAGE.md](./USAGE.md) to actually use it.

![Mycora's three-pane layout: tree on the left (with a second, read-only
mounted vault below it), a Markdown-rendered body preview with tag
badges in the middle, and backlinks on the
right](./docs/screenshot.png)

---

## The problem

Two note-taking philosophies dominate today, and almost nothing in the
terminal does both well:

- **Hierarchical outliners** (Workflowy, Dendron, classic mind maps) give you
  a clean parent/child tree — great for structure, but notes live in
  isolation from each other unless they share an ancestor.
- **Zettelkasten / graph tools** (Obsidian, Roam, `zk`) give you free-form
  links between notes — great for discovery, but there's no strict
  hierarchy to anchor yourself in, and most terminal implementations (`zk`,
  `zk-cli`) are flat file collections navigated through fuzzy search rather
  than a real hierarchical UI.

Mycora treats these as complementary, not exclusive: **every note has one
place in a tree, and can also carry links to any other note in the forest.**
The tree gives you orientation. The links give you associative reach.

## Why "Mycora"?

In a forest, trees look like separate individuals above ground, but their
root systems are frequently interconnected below ground through mycelial
networks — fungal threads that let physically distinct trees exchange
resources and signals. Foresters call this the "wood-wide web."

That's the exact shape of this project's data model:

- Each note tree is a **trunk** — a self-contained hierarchy you can
  collapse, expand, move, and reorganize like a mind map.
- Cross-references between notes, wherever they live in whichever tree, are
  the **mycelial links** — the hidden network that connects distinct
  hierarchies without flattening them into one another.

The name **Mycora** (from *mycorrhiza*, the symbiotic fungus-root
relationship) was chosen deliberately over more obvious options — most of
the literal mycology vocabulary (`mycel`, `mycelia`, `hypha`, `hyphae`,
`mycorrhiza`) was already taken on crates.io, several by adjacent
note-taking tooling. Mycora is free, short, and — more importantly — it
names the part of the design that's actually differentiating: not the tree
(everyone has trees), but the network that links separate trees together.

## Core principles

- **The tree is the skeleton, links are the nervous system.** Every note has
  exactly one parent (or is a root), full stop. Cross-links are a separate,
  many-to-many relation on top — never a substitute for structure.
- **Plain text is the source of truth.** Notes are stored as Markdown files
  with YAML frontmatter. Nothing about your data should require Mycora to
  remain readable.
- **The index is disposable.** A local database (tree position, backlinks,
  full-text index) is derived from the Markdown files and can be rebuilt
  from scratch at any time. You can always `rm` it and regenerate.
- **Keyboard-first, no compromises.** No mouse-required interactions. Modal
  navigation inspired by vim, consistent across every view.
- **Search is a feature, not an afterthought.** Full-text search should
  return relevant results fast, with ranking that reflects relevance
  (BM25-class scoring), not just substring matches.

## Features

- **Tree operations**: create, rename, delete (the whole subtree moves to
  `.trash/`, never erased outright), move/reparent with cycle detection,
  reorder siblings, deep-copy a subtree (fresh ids, no shared identity).
- **Cut, paste, and cross-vault copy**: mark a note/subtree with `x`
  (move) or `c` (copy — works from a read-only mounted vault too), then
  `p` on any destination to complete it as that note's last child.
- **Tags**: `:tag add <tag>` / `:tag del <tag>` on the selected note,
  shown as `#tag` badges along the bottom of the body preview; undo/redo-
  aware, a no-op (not an error) on a duplicate add or a missing removal.
- **Undo/redo**: every structural operation, body edit, and tag change is
  reversible for the rest of the session, built on inverses computed
  against the live tree, not frozen snapshots.
- **Local-first storage**: Markdown + YAML frontmatter is the sole source
  of truth; malformed files, duplicate ids, orphaned parents, and even a
  note listing itself as its own parent are self-healed with a warning
  rather than causing a crash or data loss. Every write (a note,
  `config.toml`, `session.toml`) is atomic — a crash or power loss
  mid-write can't leave a truncated file behind.
- **Full-text search**: SQLite FTS5 over titles + bodies, BM25-ranked
  with snippets, plus faceted filtering by tag/date/branch; a live `/`
  search overlay in the TUI, `:tags`/`:tags list` for tag-only browsing;
  `mycora reindex --watch` keeps the index in sync as files change on
  disk. Scales linearly to thousands of notes — see
  [BENCHMARK.md](./BENCHMARK.md).
- **Cross-links**: `[[wikilink]]`-style references between any two notes,
  independent of tree position, resolved across mounted vaults; a
  backlinks panel per note; ambiguous titles fan out to a link per match
  rather than erroring, unresolved ones surface as broken-link warnings.
- **Multi-vault mounting**: a registry of named vaults, exactly one
  editable (`"default"`) at a time and every other mounted vault
  read-only but fully navigable. An unmounted vault still shows up in
  the tree as its own placeholder row (`⊘`), and can be compressed to a
  single archive file to reclaim disk space (`▦` row, `mycora vault
  archive`/`unarchive`) — either row category can be hidden with
  `:config unmount/archive show/hide`. A `mycora vault` CLI
  (`add`/`init`/`rename`/`promote`/`mount`/`unmount`/`archive`/
  `unarchive`/`remove`/`list`/`sync-filenames`) manages the registry.
- **A three-pane layout**: resizable tree + Markdown-rendered body
  preview + backlinks panes, a full-pane body editor, a `:` command
  palette, light/dark theming for free via named ANSI colors, and
  session persistence (remembers where you were, per vault).
- **Multilingual interface**: English (default), French, Spanish, and
  German — `language = "fr"` in `config.toml`, or `:lang <en|fr|es|de>`
  to switch live from inside the TUI, persisted for next time.
  Keybindings and command syntax stay identical in every language, like
  vim's `:w`; every string is embedded and compile-checked, so a
  missing translation is a build failure, not a runtime gap.
- **Import/export**: `mycora import` converts an existing Obsidian vault
  (folder structure becomes tree structure); `:export`/`mycora export`
  flattens a note's subtree to a single Markdown *or* PDF document
  (format inferred from the output path's extension).
- **Link autocompletion**: typing `[[` in the body editor opens a popup
  of matching note titles across every mounted vault — `Up`/`Down` to
  pick, `Tab`/`Enter` to accept, `Esc` to keep typing manually.
- **Attaching files**: `Ctrl+A` while editing a note's body copies a
  file into `attachments/` and inserts a link at the cursor — never
  rendered inline, just kept alongside the note and linked from it.

## Architecture

```
┌─────────────────────────────┐
│   ratatui + crossterm TUI   │   ← tree/body/backlinks panes, command palette
├─────────────────────────────┤
│      Mycora core (Rust)     │   ← tree ops, link graph, undo/redo
├──────────────┬──────────────┤
│  Markdown +  │   SQLite     │   ← source of truth │ derived, disposable
│  frontmatter │   (FTS5)     │     index
│  files       │              │
└──────────────┴──────────────┘
```

## Tech stack

In use today:

- **ratatui** + **crossterm** — terminal UI rendering and input
- **ratatui-textarea** — the full-pane body editor
- **pulldown-cmark** — Markdown parsing, driving both wikilink extraction
  and the rendered body preview pane
- **serde** + **serde_yaml** + **toml** — frontmatter and config
  (de)serialization
- **rusqlite** (`bundled`) — the disposable SQLite index behind full-text
  search, tag filtering, and cross-links, no system libsqlite3 dependency
- **notify** — filesystem watching for `mycora reindex --watch`
- **markdown2pdf** — renders a flattened subtree to a paginated PDF for
  `:export`/`mycora export` when the output path ends in `.pdf`
- **tar** + **flate2** — `mycora vault archive`/`unarchive`'s `.tar.gz`
  compression, both pure-Rust (`flate2` defaults to its `miniz_oxide`
  backend, no system zlib needed)
- **uuid**, **time**, **anyhow**, **clap** — note ids, timestamps, error
  handling, CLI parsing

Considered and deliberately not adopted:

- **tantivy** — the goal it would have served (relevance-ranked search)
  is already met by FTS5's own BM25 `rank` and `snippet()` support, so a
  second full-text engine was set aside rather than added on spec; it
  stays an option if a concrete gap shows up (typo tolerance, ranking
  quality at large vault sizes) rather than something adopted upfront.

## How Mycora compares

| | Structure | Cross-links | Storage | Search | Interface |
|---|---|---|---|---|---|
| **Mycora** | strict tree | yes | Markdown + SQLite index | FTS5, BM25-ranked | TUI |
| Obsidian | free-form graph | yes | Markdown | plugin-dependent | GUI |
| `zk` (Go) | flat, tag/link-based | yes | Markdown | fzf-based | CLI |
| `zk-cli` (Rust) | flat, tag/link-based | yes | Markdown | fuzzy (skim) | CLI |
| Dendron | hierarchical (dot notation) | yes | Markdown | plugin-dependent | VS Code |
| `tmmpr` | free canvas (x/y) | visual only | proprietary | none | TUI |

## Status

Working and daily-usable, published on crates.io. Configurable
keybindings are the one thing deliberately left out for now, until real
friction shows up in practice rather than being built speculatively.
See [USAGE.md](./USAGE.md) for how to use it today, and
[BENCHMARK.md](./BENCHMARK.md) for how it performs at thousands of
notes.

## Showcase vaults

`examples/` ships real, committed Mycora vaults you can mount and browse
rather than just read about — a working example of the on-disk file
format, and a better tour of the tree-plus-links model than any
screenshot:

- **`showcase-vault/`** — Mycora documenting itself: philosophy,
  interface, features, and the specific design decisions behind them,
  as interlinked notes.
- **`showcase-jsoned/`**, **`showcase-rowdy/`**, **`showcase-terapi/`** —
  the same treatment applied to the sibling projects below, built from
  each one's own docs (not filler content), and a good demonstration of
  cross-vault linking and read-only secondary vaults at once.

Mount any of them alongside your own vault with, e.g.:

```sh
mycora vault add jsoned ./examples/showcase-jsoned
```

## License

Dual-licensed under [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE),
at your option.

## Other projects

Other terminal tools from the same author (see their own showcase vault
above for a deeper look at each):

- **[rowdy](https://github.com/TSODev/rowdy)** — a fast, modern TUI
  database management tool (`ratatui` + `sqlx`), for inspecting,
  querying, and managing databases without leaving the terminal.
- **[terapi](https://github.com/TSODev/terapi)** — a keyboard-driven TUI
  for exploring, testing, and automating REST and GraphQL APIs, with a
  headless campaign runner.
- **[jsoned](https://github.com/TSODev/jsoned)** — a keyboard-driven TUI
  for viewing and editing JSON, with full structural editing, undo/redo,
  search, and conversion to/from YAML, TOML, CSV, and JSONL.
