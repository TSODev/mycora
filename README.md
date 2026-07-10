# Mycora

**A tree-native, mycelium-linked note-taking TUI, written in Rust.**

Mycora is a terminal application for building and navigating hierarchical
notes — mind-map-style trees — while letting individual notes reference each
other across branches, the way a mycelial network links the root systems of
otherwise separate trees.

> Status: working, in active development. Hierarchical notes, Markdown
> persistence, full tree editing with undo/redo, and SQLite-backed
> full-text search are all shipped today (v0.1–v0.4). The "mycelial"
> cross-link half of the name — `[[wikilink]]`-style references between
> notes — hasn't landed yet; see [ROADMAP.md](./ROADMAP.md) for what's
> built vs. still ahead, and [USAGE.md](./USAGE.md) to actually use it.

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

## Built (v0.1–v0.4)

- **Tree operations**: create, rename, delete (the whole subtree moves to
  `.trash/`, never erased outright), move/reparent with cycle detection,
  reorder siblings, deep-copy a subtree (fresh ids, no shared identity).
- **Undo/redo**: every structural operation is reversible for the rest of
  the session.
- **Local-first storage**: Markdown + YAML frontmatter is the sole source
  of truth; malformed files, duplicate ids, and orphaned parents are
  self-healed with a warning rather than causing a crash or data loss.
- **Full-text search**: SQLite FTS5 over titles + bodies, with a live `/`
  search overlay in the TUI; `mycora reindex --watch` keeps the index in
  sync as files change on disk.
- **Tag filtering**: AND/OR set-filtering over tags — index/API level only
  so far, no TUI command yet.

## Still ahead

- **Cross-links**: `[[wikilink]]`-style references between any two notes,
  independent of tree position; backlinks panel per note.
- **Copy-as-link**: today's `y` always deep-copies; a link-only copy is
  really a cross-link with tree presence, so it waits on cross-links
  landing (the index's `links` table already exists, just unused until
  then).
- **Relevance-ranked search**: upgrading from FTS5 to tantivy/BM25 scoring.
- **A richer layout**: split-pane (tree + body + backlinks), theming, a
  command palette.
- **Import/export**: Obsidian-style vault import, flattened Markdown
  export.

See [ROADMAP.md](./ROADMAP.md) for the full staged plan.

## Planned architecture

```
┌─────────────────────────────┐
│   ratatui + crossterm TUI   │   ← tree view, editor pane, search overlay
├─────────────────────────────┤
│      Mycora core (Rust)     │   ← tree ops, link graph, undo/redo
├──────────────┬──────────────┤
│  Markdown +   │   SQLite     │   ← source of truth │ derived index
│  frontmatter  │   (+ FTS/    │
│  files        │   tantivy)   │
└──────────────┴──────────────┘
```

## Tech stack

In use today:

- **ratatui** + **crossterm** — terminal UI rendering and input
- **serde** + **serde_yaml** + **toml** — frontmatter and config
  (de)serialization
- **rusqlite** (`bundled`) — the disposable SQLite index behind full-text
  search and tag filtering, no system libsqlite3 dependency
- **notify** — filesystem watching for `mycora reindex --watch`
- **uuid**, **time**, **anyhow**, **clap** — note ids, timestamps, error
  handling, CLI parsing

Not adopted yet, candidates for later stages:

- **tantivy** — upgrade path for full-text search once FTS5's ranking
  proves insufficient (v0.6)
- **pulldown-cmark** — Markdown parsing, for wikilink extraction (v0.5)
  and rendering the note body pane (v0.7)

## How Mycora compares

| | Structure | Cross-links | Storage | Search | Interface |
|---|---|---|---|---|---|
| **Mycora** | strict tree | planned (v0.5) | Markdown + SQLite index | FTS5 (tantivy planned) | TUI |
| Obsidian | free-form graph | yes | Markdown | plugin-dependent | GUI |
| `zk` (Go) | flat, tag/link-based | yes | Markdown | fzf-based | CLI |
| `zk-cli` (Rust) | flat, tag/link-based | yes | Markdown | fuzzy (skim) | CLI |
| Dendron | hierarchical (dot notation) | yes | Markdown | plugin-dependent | VS Code |
| `tmmpr` | free canvas (x/y) | visual only | proprietary | none | TUI |

## Status

Working and daily-usable — v0.1 through v0.4 are done: in-memory tree,
Markdown persistence, full structural operations (move, copy, reorder,
delete with confirmation and a trash, undo/redo), and SQLite-backed search
(FTS5 full-text, tag filtering, `mycora reindex --watch`). See
[USAGE.md](./USAGE.md) for how to use it today, and
[ROADMAP.md](./ROADMAP.md) for what's still ahead (cross-links, a richer
layout, import/export) through to a stable v1.0.

## License

Dual-licensed under [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE),
at your option.
