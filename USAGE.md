# Mycora — Usage Guide

> Reflects what's actually implemented today (v0.1–v0.3): an in-memory tree
> with Markdown persistence and full structural operations. No search, no
> cross-links/wikilinks, no multi-pane layout yet — see
> [ROADMAP.md](./ROADMAP.md) for what's still ahead.

## Table of Contents

- [Installation](#installation)
- [Launching Mycora](#launching-mycora)
- [Configuration](#configuration)
- [The vault format](#the-vault-format)
- [Layout](#layout)
- [Creating and renaming notes](#creating-and-renaming-notes)
- [Moving notes](#moving-notes)
- [Reordering siblings](#reordering-siblings)
- [Copying notes](#copying-notes)
- [Deleting notes and the trash](#deleting-notes-and-the-trash)
- [Undo and redo](#undo-and-redo)
- [Keybinding reference](#keybinding-reference)

---

## Installation

```sh
cargo install mycora
```

## Launching Mycora

```sh
mycora
```

No arguments yet. Mycora opens whichever vault is configured (see
[Configuration](#configuration)) and creates it, with a starter "Welcome to
Mycora" note, if it doesn't exist yet. Press `q` twice to quit (a stray
single `q` won't close the app).

## Configuration

Config file at `~/.config/mycora/config.toml`:

```toml
vault_path = "/path/to/your/notes"
```

If the file is missing, or `vault_path` isn't set, Mycora defaults to
`~/mycora`.

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
- `tags` — not yet exposed in the TUI (planned for v0.4's tag filter).
- The title lives in the body as the first `# Heading`, not in frontmatter.

Malformed files, duplicate ids, or a note whose `parent` can't be found are
self-healed (or skipped, if unreadable) with a warning printed before the
TUI starts — nothing is ever silently lost or causes a crash.

## Layout

A single pane: an indented, collapsible tree of notes, with a one-line
status bar at the bottom showing the current mode and the relevant
keybinding hints. A richer split-pane layout (note body, backlinks) is
planned for v0.7.

## Creating and renaming notes

- `a` — new child of the selected note
- `o` — new sibling, right after the selected note
- A freshly created note opens an empty naming prompt — type the title,
  `Enter` to confirm. Pressing `Esc` cancels the *naming*, not the note
  itself — it's kept, titled "New note", ready to rename later with `i`.
- `i` — rename the selected note (prefills its current title so you can
  edit it, rather than starting blank)

## Moving notes

- `Tab` — indent: reparents the selected note under its immediately
  preceding sibling
- `Shift+Tab` — outdent: reparents the selected note as a sibling of its
  current parent

Reparenting to an arbitrary note (not just a neighbor) needs a note picker,
which depends on the search overlay — planned for v0.4 onward.

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
| `y` | Copy selected note (deep-copy) |
| `Tab` | Indent (reparent under previous sibling) |
| `Shift+Tab` | Outdent (reparent as sibling of parent) |
| `K` | Move up among siblings |
| `J` | Move down among siblings |
| `d` | Delete (asks for confirmation) |
| `u` | Undo |
| `Ctrl+R` | Redo |
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
