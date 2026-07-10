# Mycora — Usage Guide

> Reflects what's actually implemented today (v0.1–v0.6, plus a start on
> v0.7): an in-memory tree with Markdown persistence, full structural
> operations, SQLite-backed search (with snippets and faceted filters) and
> tag filtering, read-only multi-vault mounting, `[[wikilink]]` cross-links
> (including cross-vault ones) with a backlinks panel and link-count
> badges, a full-pane note-body editor, and a three-pane layout (tree,
> rendered-Markdown body preview, backlinks). No pane resizing, link
> autocompletion, or configurable keybindings/theming yet — see
> [ROADMAP.md](./ROADMAP.md) for what's still ahead.

## Table of Contents

- [Philosophy & vocabulary](#philosophy--vocabulary)
- [Installation](#installation)
- [Launching Mycora](#launching-mycora)
- [Configuration](#configuration)
- [The vault format](#the-vault-format)
- [The search index](#the-search-index)
- [Layout](#layout)
- [Searching](#searching)
- [Creating and renaming notes](#creating-and-renaming-notes)
- [Editing a note's body](#editing-a-notes-body)
- [Moving notes](#moving-notes)
- [Reordering siblings](#reordering-siblings)
- [Copying notes](#copying-notes)
- [Deleting notes and the trash](#deleting-notes-and-the-trash)
- [Undo and redo](#undo-and-redo)
- [Keybinding reference](#keybinding-reference)

---

## Philosophy & vocabulary

Mycora treats two note-taking philosophies — strict hierarchy and free-form
cross-referencing — as complementary rather than an either/or choice; see
the [README](./README.md#the-problem) for the full rationale. The short
version:

- **The tree is the skeleton.** Every note has exactly one parent, or is a
  root — a strict hierarchy, no exceptions, so a note always has one
  unambiguous place you can navigate back to.
- **Mycelial links are the nervous system.** Independent of tree position,
  a note can reference any other note via `[[wikilink]]`-style links —
  even one in a different mounted vault — the way a mycelial fungal
  network connects the root systems of separate trees underground. The
  tree gives you orientation; links give you associative reach. Writing
  the links themselves needs a note-body editor, which doesn't exist in
  the TUI yet (planned for v0.7) — for now, add `[[links]]` by editing a
  note's Markdown file directly, then `mycora reindex` (or just reopen the
  TUI, which reindexes on startup) to resolve them.
- **Plain Markdown is the source of truth.** Every note is one `.md` file
  with YAML frontmatter (see [The vault format](#the-vault-format)) —
  nothing Mycora keeps in its own state is authoritative.
- **The index is disposable.** The SQLite index behind search and tag
  filtering is entirely derived from the vault's Markdown files. `mycora
  reindex` rebuilds it from scratch at any time; deleting
  `~/.local/share/mycora/index.sqlite3` and rerunning it is always safe.

### Vocabulary

| Term | Meaning |
|---|---|
| **Vault** | A directory of Markdown note files, plus its derived SQLite index. The config can register several named vaults; every one flagged `mounted` (the default) loads at startup — see [Configuration](#configuration). |
| **Note** | One Markdown file: YAML frontmatter (`id`, `parent`, `order`, `tags`, timestamps) plus a `# Title` heading and body. |
| **Tree** / **trunk** | The hierarchy formed by a note and all its descendants. The README's mycelial-network framing calls this a "trunk"; the code and the rest of this guide mostly just say "tree." A vault can hold several independent trees. |
| **Root** | A note with no parent — the top of one tree/trunk. |
| **Parent / child** | A note's one structural parent, or the notes directly beneath it — the relationship the tree is built from. |
| **Mycelial link** | A `[[wikilink]]`-style, many-to-many reference between two notes, independent of where either one lives in the tree — can even cross from one mounted vault into another. |
| **Index** | The disposable SQLite database (`notes`, `tree_edges`, `tags`, `notes_fts`, `links`) behind search, tag filtering, and backlinks — never a second source of truth. |

## Installation

```sh
cargo install mycora
```

## Launching Mycora

```sh
mycora            # open the configured vault
mycora --help     # usage
mycora --version  # print the version and exit
```

No other arguments yet. Mycora opens whichever vault is configured (see
[Configuration](#configuration)) and creates it, with a starter "Welcome to
Mycora" note, if it doesn't exist yet. Press `q` twice to quit (a stray
single `q` won't close the app).

Which note was selected and which branches were expanded/collapsed are
remembered per vault and restored the next time you open it — whether you
quit with `q`/`q` or `Ctrl+C`. Stored at
`~/.local/share/mycora/session.toml`, safe to delete if you ever want to
reset it.

## Configuration

Config file at `~/.config/mycora/config.toml`. Mycora keeps a registry of
named vaults, every one of which is *mounted* (loaded) at startup unless
you opt it out:

```toml
[[vaults]]
name = "default"
path = "/path/to/your/notes"

[[vaults]]
name = "work"
path = "/path/to/work/notes"

[[vaults]]
name = "archive"
path = "/path/to/old/notes"
mounted = false   # known to the registry, but not loaded
```

The entry named `default` is the one you actually work in — it's the only
vault you can create/rename/move/delete notes in today. Every other
mounted vault shows up read-only, stacked below it with a `── name ──`
separator (see [Layout](#layout)); if none is named `default`, the first
mounted entry becomes the editable one instead. `mounted` defaults to
`true` when omitted, so a vault only becomes registry-only-but-inactive if
you explicitly set `mounted = false`. Editing a non-`default` mounted
vault directly (reparenting into it, switching which one you're "in")
isn't implemented yet.

The older single-vault form is still accepted as a fallback when
`[[vaults]]` is absent:

```toml
vault_path = "/path/to/your/notes"
```

If the file is missing, or neither `[[vaults]]` nor `vault_path` is set,
Mycora defaults to a single vault at `~/mycora`.

## The vault format

Notes are plain Markdown files, one per note, in a single flat directory —
no nested folders yet. Hierarchy is expressed entirely through frontmatter,
never through file layout, so the vault stays readable and editable by hand
or with any other tool. A note file looks like:

```markdown
---
id: 23018896-2237-476e-8bd3-e8a760ae523d
parent: null
order: 0
tags: []
created: 2026-07-06T18:50:46Z
updated: 2026-07-06T18:50:46Z
---

# Note title

The note body, in Markdown.
```

- `id` — a UUID v4, generated once at creation. Stable across renames.
- `parent` — the id of the parent note, or `null` for a root note.
- `order` — position among siblings.
- `tags` — indexed for AND/OR filtering (`Index::filter_by_tags`), but not
  yet exposed as a TUI command; only editable by hand in the file for now.
- The title lives in the body as the first `# Heading`, not in frontmatter.

Malformed files, duplicate ids, or a note whose `parent` can't be found are
self-healed (or skipped, if unreadable) with a warning printed before the
TUI starts — nothing is ever silently lost or causes a crash.

## The search index

Alongside the vault(s), Mycora keeps a single disposable SQLite index at
`~/.local/share/mycora/index.sqlite3` shared across every mounted vault
(each table is keyed by vault name) — it powers full-text search
([Searching](#searching)), backlinks, link-count badges, and tag
filtering. It's derived entirely from the vaults' Markdown files, never
authoritative: safe to delete, safe to rebuild, never a second copy of
your data.

You generally don't need to manage this yourself — the TUI reindexes
every mounted vault automatically on startup, and reindexes the default
vault again every time you open search with `/` or backlinks with `b`, so
results always reflect it as loaded (including edits made earlier in the
same session). The CLI commands below are for headless use: rebuilding
the index without opening the TUI, or keeping it warm in the background
for some other tool to query directly.

```sh
mycora reindex          # rebuild every mounted vault once, then exit
mycora reindex --watch  # rebuild, then keep running and rebuild again
                         # whenever a file in any mounted vault changes
```

`--watch` debounces bursts of filesystem events (300ms) into a single
reindex, since one save is often a write followed by a rename-into-place.
It watches every mounted vault's directory non-recursively, matching how
each vault itself is a flat directory. Stop it with `Ctrl+C`.

Each reindex is a full rebuild of the vault's rows, not a per-file diff —
intentionally: the index is small and disposable enough that regenerating
it wholesale is simpler and safer than trying to patch it incrementally.

### Tags

Every note's `tags` (see [The vault format](#the-vault-format)) are
indexed for AND/OR set-filtering — a note matching *all* of a set of tags,
or *any* of them. There's no TUI command or CLI flag for this yet; it's
only reachable through the Rust API (`Index::filter_by_tags`) for now. See
[ROADMAP.md](./ROADMAP.md) for when a user-facing surface for it lands.

## Layout

Three columns, plus a 2-line status bar at the bottom: the top row is a
breadcrumb (`vault › branch › note`) for the selected note; the bottom row
shows the current mode and the relevant keybinding hints (`key: label`,
key in bold). A prompt — the delete confirmation, the quit-confirm notice,
or an error — replaces the bottom row only, leaving the breadcrumb above
it in place.

- **Tree** (left) — the indented, collapsible note tree, same as before.
  If other vaults are mounted alongside the default one (see
  [Configuration](#configuration)), their root notes appear stacked below
  it, each vault preceded by a dimmed `── name ──` separator. These rows
  are read-only: `j`/`k` never selects into them, and their link-count
  badges work the same as the default vault's, just computed against that
  vault's own notes.
- **Body preview** (middle) — the selected note's body, rendered as
  Markdown (headings, bold/italic, inline/block code, lists, blockquotes,
  horizontal rules). Updates live as you move the selection. Read-only and
  not interactive: links and `[[wikilinks]]` render as plain text, not as
  something you can click or navigate from the preview itself.
- **Backlinks** (right) — notes linking to the selected note, also live.
  Read-only glance list: jumping to one of them still goes through the
  interactive backlinks overlay (`b`), not this pane directly.

Column widths are fixed (40%/40%/20%) for now — interactive resizing is
still an open item.

Search (`/`), the backlinks overlay (`b`), and the body editor (`e`) all
still take over the whole screen as full-pane overlays rather than living
inside these columns.

## Searching

- `/` — opens a search prompt over the active vault's title + body text
- Results update as you type: each word becomes a prefix match (`arch`
  matches "Architecture"), and every word you've typed must match
  somewhere in the title or body — not a raw substring search, and not
  fuzzy/typo-tolerant yet
- Each result shows its title plus a snippet of body text around the
  match, with the matched word or phrase highlighted
- `↑` / `↓` — move between results
- `Enter` — jump to the selected result: expands its ancestors so it's
  visible, selects it in the tree, and returns to Normal mode
- `Esc` — cancels, returning to Normal mode without changing your current
  selection

Opening search always reindexes first (see [The search index](#the-search-index)),
so results reflect the tree exactly as it stands, including edits you
haven't run `mycora reindex` for yet. Search and backlinks (`b`) only
cover the default (editable) vault — other mounted vaults are read-only
and don't have anywhere for a jump-to-result to land yet, so they're left
out of both, even though they're indexed and their link-count badges work
(see [Layout](#layout)).

## Creating and renaming notes

- `a` — new child of the selected note
- `o` — new sibling, right after the selected note
- A freshly created note opens an empty naming prompt — type the title,
  `Enter` to confirm. Pressing `Esc` cancels the *naming*, not the note
  itself — it's kept, titled "New note", ready to rename later with `i`.
- `i` — rename the selected note (prefills its current title so you can
  edit it, rather than starting blank)

## Editing a note's body

- `e` — opens the selected note's Markdown body in a full-pane editor,
  loaded with whatever text it already has
- Type normally — multi-line, `Enter` inserts a newline (it doesn't
  confirm/exit, unlike renaming)
- `Esc` — saves and returns to Normal mode. There's no separate
  discard-without-saving: if you want to back out of an edit after the
  fact, `u` in Normal mode undoes the whole session as one step
  (see [Undo and redo](#undo-and-redo))
- An edit session that changes nothing doesn't write to disk or create an
  undo entry

Not rendered as formatted Markdown, and not split alongside the tree yet
— both are separate, still-open items (see [ROADMAP.md](./ROADMAP.md)).
This is also what unblocks writing `[[wikilinks]]` from inside the TUI
instead of editing the file by hand (see [The search
index](#the-search-index)) — though autocompleting them as you type isn't
implemented yet.

## Moving notes

- `Tab` — indent: reparents the selected note under its immediately
  preceding sibling
- `Shift+Tab` — outdent: reparents the selected note as a sibling of its
  current parent

Reparenting to an arbitrary note (not just a neighbor) needs a note
picker. The [search overlay](#searching) it depends on now exists, but
the picker itself — reusing search to choose a reparent target rather
than just jump to a note — isn't built yet.

## Reordering siblings

- `K` — move the selected note up among its siblings
- `J` — move it down

## Copying notes

- `y` — deep-copies the selected note and its whole subtree as a new
  sibling right after it, with fresh ids and timestamps. This is always a
  real duplicate, never a live reference to the original — see
  ROADMAP.md's now-resolved copy-semantics question.

## Deleting notes and the trash

- `d` — asks for confirmation (`y`/`n`) before deleting
- Confirming removes the selected note *and all of its descendants
  together*, moving every removed file into `<vault>/.trash/` rather than
  erasing it. Trash is never auto-emptied — recoverable by hand if needed.

## Undo and redo

- `u` — undo the last action
- `Ctrl+R` — redo

Covers renames, moves, reorders, copies, and deletes, for the rest of the
session. Not persisted across restarts.

## Keybinding reference

### Normal mode

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `l` / `→` / `Enter` | Expand |
| `h` / `←` | Collapse |
| `Space` | Toggle expand/collapse |
| `a` | New child note |
| `o` | New sibling note |
| `i` | Rename selected note |
| `e` | Edit body (see [Editing a note's body](#editing-a-notes-body)) |
| `y` | Copy selected note (deep-copy) |
| `Tab` | Indent (reparent under previous sibling) |
| `Shift+Tab` | Outdent (reparent as sibling of parent) |
| `K` | Move up among siblings |
| `J` | Move down among siblings |
| `d` | Delete (asks for confirmation) |
| `u` | Undo |
| `Ctrl+R` | Redo |
| `/` | Open search (see [Searching](#searching)) |
| `q` `q` | Quit (press twice — any other key cancels) |
| `Ctrl+C` | Quit immediately — bypasses any prompt or confirmation |

### Naming / renaming

| Key | Action |
|---|---|
| *(type)* | Edit the title |
| `Enter` | Confirm |
| `Esc` | Cancel (see [Creating and renaming notes](#creating-and-renaming-notes)) |

### Delete confirmation

| Key | Action |
|---|---|
| `y` / `Enter` | Confirm delete |
| `n` / `Esc` | Cancel |

### Search

| Key | Action |
|---|---|
| *(type)* | Filter results (see [Searching](#searching)) |
| `↑` / `↓` | Move between results |
| `Enter` | Jump to the selected result |
| `Esc` | Cancel, keeping the current selection |

### Edit body

| Key | Action |
|---|---|
| *(type)* | Edit the body, multi-line — `Enter` inserts a newline |
| `Esc` | Save and return to Normal mode (see [Editing a note's body](#editing-a-notes-body)) |
