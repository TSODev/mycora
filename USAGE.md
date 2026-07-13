# Mycora — Usage Guide

> Reflects what's actually implemented today (v0.1–v0.9): an in-memory
> tree with Markdown persistence and atomic writes throughout, full
> structural operations with undo/redo, tag management, SQLite-backed
> search (with snippets and faceted filters) that scales linearly to
> thousands of notes, multi-vault mounting (read-only secondary vaults,
> unmounted and archived vaults both visible as their own tree rows),
> `[[wikilink]]` cross-links (including cross-vault ones) with a
> backlinks panel, link-count badges, and autocompletion while typing, a
> full-pane note-body editor, a resizable three-pane layout (tree,
> rendered-Markdown body preview, backlinks) with light/dark-aware
> colors, a multilingual interface (English/French/Spanish/German), and
> a `:` command palette (`:reindex`, `:tags`, `:tag`, `:lang`, `:panes`,
> `:export`, `:config`, `:q`). No configurable keybindings yet — see
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
- [Backlinks](#backlinks)
- [Command palette](#command-palette)
- [Exporting a subtree](#exporting-a-subtree)
- [Importing an Obsidian vault](#importing-an-obsidian-vault)
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
  tree gives you orientation; links give you associative reach. Write
  `[[links]]` right in the TUI with the full-pane body editor (`e`, see
  [Editing a note's body](#editing-a-notes-body)), or by editing a note's
  Markdown file directly outside Mycora — either way, `mycora reindex`
  (or just reopening the TUI, which reindexes on startup) resolves them.
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
remembered per vault, and pane widths (see [Layout](#layout)) are
remembered too — all restored the next time you open Mycora, whether you
quit with `q`/`q` or `Ctrl+C`. Stored at
`~/.local/share/mycora/session.toml`, safe to delete if you ever want to
reset it.

**Want to try it before pointing Mycora at your own notes?**
[`examples/showcase-vault/`](./examples/showcase-vault/) is a real Mycora
vault — Mycora's own philosophy, interface, and design decisions, written
as interlinked notes with tags, built with this exact tool. Point a
registry entry at it (see [Configuration](#configuration)) to explore
search, backlinks, and the command palette against real content instead
of an empty vault.

## Configuration

Config file at `~/.config/mycora/config.toml`. Mycora keeps a registry of
named vaults, every one of which is *mounted* (loaded) at startup unless
you opt it out:

```toml
language = "fr"   # optional TUI language: "en" (default), "fr", "es", or "de"

[[vaults]]
name = "default"
path = "/path/to/your/notes"

[[vaults]]
name = "work"
path = "/path/to/work/notes"

[[vaults]]
name = "old-notes"
path = "/path/to/old/notes"
mounted = false   # known to the registry, but not loaded

[[vaults]]
name = "backup-2024"
path = "/path/to/backup-2024"
archived = "/path/to/backup-2024.tar.gz"   # compressed; path above doesn't exist right now
```

The entry named `default` is the one you actually work in — it's the only
vault you can create/rename/move/delete notes in today. Every other
mounted vault is fully browsable (not just displayed) but read-only,
stacked below it with a `── name ──` separator (see [Layout](#layout));
if none is named `default`, the first mounted entry becomes the editable
one instead. `mounted` defaults to `true` when omitted, so a vault only
becomes registry-only-but-inactive if you explicitly set
`mounted = false`. `archived`, present only on a vault `mycora vault
archive` has compressed (see [below](#registering-a-vault-from-the-cli)),
holds the archive file's path — always implies `mounted = false`, since
there's nothing left at `path` to load until it's unarchived. Editing a
non-`default` mounted vault directly (reparenting into it, switching
which one you're "in") isn't implemented
yet.

A `mounted = false` entry isn't invisible, either: it shows up in the
tree as its own `⊘ name` row (dark gray, no fold marker — nothing is
loaded for it, so there's nothing to expand). Selecting it shows the
vault's path and the exact `mycora vault mount` command to bring it
back, in place of a note body; every mutating key (including fold) is a
no-op there, and the breadcrumb's corner marker reads `UNMOUNTED`
instead of `READ-ONLY`. An [archived](#registering-a-vault-from-the-cli)
vault gets a `▦ name` row instead — same idea, but pointing at `mycora
vault unarchive` and marked `ARCHIVED`, since there's nothing at its
path to mount until it's unarchived first. Either category can be
hidden from the tree entirely with `:config unmount hide`/`:config
archive hide` (see [Command palette](#command-palette)) if a registry
with several of either gets cluttered.

`language` switches every label, hint, prompt, and status message in the
TUI. Four are built in: English (`"en"`, the default), French (`"fr"`),
Spanish (`"es"`), and German (`"de"`) — the last two machine-translated
and not yet reviewed by a native speaker, unlike English/French; open an
issue if something reads wrong. What deliberately does *not* translate:
keybindings, command names and their arguments (`:tags limit`,
`show`/`hide`, ...), the `mycora` CLI's own output, and anything
persisted in your notes — interface syntax stays identical everywhere,
like vim's `:w`, so documentation and muscle memory work in any
language. (The welcome note auto-created in an empty vault is the one
exception: it's stamped in whichever language is configured at creation
time, and being note content, stays as written.) An unrecognized code
fails at startup with a clear error rather than silently falling back to
English. Every language ships inside the binary — there are no separate
language files to install or keep in sync. You can also switch live from
inside the TUI with `:lang <en|fr|es|de>` (see
[Command palette](#command-palette)), which writes this same key back
here for you.

The older single-vault form is still accepted as a fallback when
`[[vaults]]` is absent:

```toml
vault_path = "/path/to/your/notes"
```

If the file is missing, or neither `[[vaults]]` nor `vault_path` is set,
Mycora defaults to a single vault at `~/mycora`.

### Registering a vault from the CLI

```sh
mycora vault add <name> <path>              # adds it, mounted
mycora vault add <name> <path> --no-mount   # adds it, registry-only
```

Appends a `[[vaults]]` entry to `config.toml`, creating the file (and its
parent directory) if neither exists yet. Errors rather than overwriting if
`name` is already registered — remove the old entry by hand first if
that's what you want. If the file only had the older single-vault
`vault_path` form, that vault is migrated into an explicit `"default"`
registry entry first, so adding a second vault doesn't silently drop it.
The path doesn't need to exist yet; Mycora creates it on first use, same
as pointing the TUI at a brand-new `vault_path` already does.

Rewrites the whole file from a fresh parse (like `cargo add` rewriting
`Cargo.toml`) rather than a surgical text edit — simple, but any hand-added
comments or unusual formatting in `config.toml` won't survive.

To also create the vault directory and mount it in one step:

```sh
mycora vault init <name> <path>
```

Same as `vault add` (always mounted, no `--no-mount` option — mounting is
the point), plus creating `<path>` if it doesn't exist yet. Reports
whether the new vault actually became the *active* (read-write) one:
that only happens if it's named `"default"`, or ends up the only/first
mounted entry (see [Configuration](#configuration) above) — if another
vault is already named `"default"`, the new one is still created and
mounted, just read-only in the TUI, and the command tells you so rather
than silently renaming the existing `"default"` entry to make room.

To fix that afterward (or to switch which vault you're editing at any
other time):

```sh
mycora vault rename <old-name> <new-name>   # renames a registry entry
mycora vault promote <name>                 # makes it the active vault
```

`rename` only changes the name — path and mount state are untouched.
`promote` makes `<name>` active by renaming it to `"default"`, the exact
name [Configuration](#configuration) says `active_vault` looks for; it
*refuses* if a different vault already holds that name rather than
reassigning it automatically, so the usual fix-up sequence is:

```sh
mycora vault rename default old-default   # free up "default"
mycora vault promote work                 # work becomes the new default
```

Both commands are no-ops (no error, no file change) if there's nothing
to do — renaming a vault to its own name, or promoting one that's
already `"default"`.

To flag a registered vault in or out of loading at startup, without
removing it from the registry entirely:

```sh
mycora vault mount <name>
mycora vault unmount <name>
```

Also no-ops if the vault's already in the requested state. Unmounting
every vault in the registry (including the active one) doesn't break
anything — `active_vault` still self-heals to some vault, and Mycora
loads it regardless of its `mounted` flag, so the app always starts.

To reclaim disk space for a vault you don't need mounted right now:

```sh
mycora vault archive <name>
mycora vault unarchive <name>
```

`archive` compresses the vault's directory into a single `<name>.tar.gz`
next to it (or wherever you pass as a second argument) and **removes the
original directory** — the archive is read back and verified before
anything is deleted, so a corrupt archive is caught while the original
still exists rather than only discovered after. Refuses on a vault
that's still mounted (`mycora vault unmount <name>` first — archiving
something meant to still be live would pull the rug out from under it)
or one that's already archived. `unarchive` reverses it: restores the
directory from the archive and removes the archive file, but leaves the
vault unmounted — `mycora vault mount <name>` afterward is a separate,
explicit step. Both are CLI-only, same as every other `vault ...`
subcommand — there's no `:` equivalent, since archiving acts on the
registry rather than on anything currently open in the TUI. An archived
vault still shows up in the TUI, as its own `▦ name` row — see
[Layout](#layout) and `:config archive show/hide` under
[Command palette](#command-palette).

To see everything currently registered:

```sh
mycora vault list
```

Prints each vault's name, path, and status tags: `active` if it's the
one you edit in, plus exactly one of `mounted`/`not mounted`/`archived`
(e.g. `[active, mounted]`). To unregister one:

```sh
mycora vault remove <name>
```

**Only ever removes the `config.toml` entry — the vault's Markdown
files on disk are never touched.** Refuses outright on `"default"`,
since that's the vault Mycora treats as active; free up the name first:

```sh
mycora vault rename default old-default   # or: mycora vault promote <other-name>
mycora vault remove old-default
```

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
- `tags` — a YAML list of strings, `[]` if none; indexed for AND/OR
  filtering (`Index::filter_by_tags`, see [Tags](#tags)) and shown as
  `#tag` badges in the body preview (see [Layout](#layout)). Manage them
  in the TUI with `:tag add <tag>`/`:tag del <tag>`, or by hand in the
  file directly — either way, `mycora reindex` picks up the change.
- `created` / `updated` — RFC 3339 timestamps (UTC). `created` is set
  once, at creation. `updated` refreshes on a rename, body edit, or tag
  change — not on a move or reorder, which change *where* the note sits
  but not the note's own content.
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
every mounted vault automatically on startup, and reindexes every
mounted vault again every time you open search with `/`, so results
always reflect them as loaded (including edits made earlier in the same
session). "Every mounted vault" means exactly that: read-only mounted
vaults (see [Configuration](#configuration)) get indexed right alongside
the active one, not just the one you can edit — that's what backlinks,
[Layout](#layout)'s link-count badges, read-only tree navigation, and
`/` search are all reading. The backlinks pane and its link-count badges
don't trigger a reindex — they read whatever the last one resolved. The
CLI commands below are for headless use: rebuilding the index without
opening the TUI, or keeping it warm in the background for some other
tool to query directly.

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
indexed for AND/OR set-filtering — a note matching *all* of a set of
tags, or *any* of them. `:tags <tag1,tag2,...>` and `:tags list` (see
[Command palette](#command-palette)) expose the *any*-of-them (OR) case
in the TUI; *all*-of-them (AND) filtering
(`Index::filter_by_tags(..., TagFilterOp::All)`) has no user-facing
command yet, only reachable through the Rust API directly.

Unlike `/` search — scoped to whichever vault the current selection is
in (see [Searching](#searching)) — both `:tags` commands deliberately
span *every mounted vault at once* by default, active or read-only. A
tag is a tag regardless of where you happen to be looking: `:tags list`
sums a tag's note count across every mounted vault rather than showing
it once per vault, and `:tags <tag1,...>`'s results each name their own
vault (`[vault-name] Title`) since they can come from more than one at a
time — `Enter` jumps to whichever one, the same cross-vault jump
`/` search and backlinks already do.

If that gets noisy with several mounted vaults, `:tags limit
<vault-name>` narrows both commands back down to just that one vault
until `:tags unlimit` lifts it — not persisted across restarts, so it
always starts unlimited on a fresh launch. The `Tags`/`Tag results`
overlay's title always names the active scope (`Tags [all vaults]` or
`Tags [vault-name]`), so a limit is never silently in effect.

`:tag add <tag>`/`:tag del <tag>` are a different operation entirely —
they add/remove a tag on the *selected* note itself, shown as `#tag`
badges in the body preview (see [Layout](#layout)), rather than
filtering *by* tags. Like every mutating command, they're scoped to
(and refuse outside of) the active vault.

## Layout

Three columns, plus a 2-line status bar at the bottom: the top row is a
breadcrumb (`vault › branch › note`) for the selected note, with a
`READ-ONLY` marker on the right whenever that selection is in a
read-only mounted vault (`UNMOUNTED`/`ARCHIVED` instead for those
vaults' placeholder rows); the bottom row shows the current mode and the
relevant keybinding hints (`key: label`, key in bold) — while a
read-only note is selected, the hints for actions that would refuse
anyway (`a`/`o`, `y`, `Tab`/`Shift+Tab`, `K`/`J`, `i`, `e`, `d`) dim out
rather than sitting at full brightness for keys that won't do anything;
on an unmounted or archived vault's row, `h`/`l`/`Space` (fold) dims
too, since there's nothing loaded to expand. A prompt — the delete
confirmation, the quit-confirm notice, or an error — replaces the bottom
row only, leaving the breadcrumb above it in place.

- **Tree** (left, blue border) — the indented, collapsible note tree, same
  as before. If other vaults are mounted alongside the default one (see
  [Configuration](#configuration)), their notes appear stacked below it,
  each vault preceded by a dimmed `── name ──` separator. `j`/`k` and
  `l`/`h`/`Space` navigate and expand/collapse into these vaults just
  like the default one — dimmed rows to mark them read-only, but fully
  browsable, not roots-only. Their link-count badges work the same as
  the default vault's, just computed against that vault's own notes.
  Below all of that, every *unmounted* registered vault gets its own
  single `⊘ name` row and every *archived* one a `▦ name` row
  (`Color::DarkGray`, no fold marker — see [Configuration](#configuration));
  selecting one shows how to mount/unarchive it in the body preview
  instead of a note body. Either row category can be hidden entirely
  with `:config unmount hide`/`:config archive hide` (see
  [Command palette](#command-palette)).
- **Body preview** (middle, magenta border, with a little horizontal
  padding off the border since it's mostly running prose) — the selected
  note's body, rendered as Markdown (headings, bold/italic, inline/block
  code, lists, blockquotes, horizontal rules). Every line break you type
  renders as its own line, even a single Enter with no blank line after
  it — a deliberate deviation from strict CommonMark (which folds a lone
  newline into a space, requiring a blank line for a real paragraph
  break) in favor of "what you typed is what you see," since notes here
  tend to be short Enter-separated fragments rather than hard-wrapped
  prose. Updates live as you move the selection, resetting scroll to the
  top each time. `Ctrl+d`/`Ctrl+u` scroll it down/up for notes longer
  than the pane. Read-only and not interactive: links and `[[wikilinks]]`
  render as plain text, not as something you can click or navigate from
  the preview itself. A fixed one-line row along the bottom shows the
  note's tags as `#tag` badges
  (cyan), always reserved even with none — add/remove them with
  `:tag add <tag>`/`:tag del <tag>` (see
  [Command palette](#command-palette)).
- **Backlinks** (right) — notes linking to the selected note, live. No
  border color while idle; press `b` to move keyboard focus into it (cyan
  border, highlighted entry) — see [Backlinks](#backlinks) below.

Column widths start at 40%/40%/20% and are adjustable: `[`/`]` shrink/grow
the tree pane, `{`/`}` shrink/grow the backlinks pane — the body pane
always absorbs whatever width the other two give up or take, down to a
10% floor per pane. Remembered across restarts, same as your last
selected note.

Every color Mycora uses is a named terminal color, not a fixed RGB value,
so it adapts to whatever light/dark/Solarized/etc. theme your terminal
emulator is configured with — there's no separate in-app theme setting to
manage.

Search (`/`) and the body editor (`e`) still take over the whole screen as
full-pane overlays rather than living inside these columns. The backlinks
pane doesn't — `b` shifts focus onto it in place.

## Searching

- `/` — opens a search prompt over title + body text, scoped to whichever
  vault the current selection is actually in (the title bar shows which
  one, e.g. `Search [vault-name]: query`) — not always the active vault,
  so searching while browsing a read-only mounted vault searches *that*
  one instead of silently falling back. Falls back to the active vault
  if nothing's selected, or the selection is an unmounted/archived
  vault's placeholder row (see [Configuration](#configuration))
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

Opening search always reindexes every mounted vault first (see
[The search index](#the-search-index)), so results reflect the tree
exactly as it stands, including edits you haven't run `mycora reindex`
for yet — even though a single search only ever queries the one vault
it's scoped to. The backlinks pane isn't scoped this way: it follows the
current selection in *any* mounted vault, read-only included, and
jumping to a backlink can land anywhere (see [Backlinks](#backlinks)).

## Backlinks

The right-hand pane (see [Layout](#layout)) always shows notes linking to
the selected one, live — whichever vault that note is in, including a
read-only mounted one, and results can span vaults too. `b` moves
keyboard focus into it — cyan border, current entry highlighted:

- `j` / `k` (or `↑` / `↓`) — move between entries
- `Enter` — jump to the focused entry: expands its ancestors so it's
  visible, selects it in the tree, and returns focus to the tree
- `Esc` or `b` again — returns focus to the tree without changing your
  current selection

Unlike search, focusing the backlinks pane doesn't reindex first — it
reads whatever the last reindex resolved, same as the pane's live view
does when it's not focused (and same as the link-count badges).

## Command palette

`:` in Normal mode opens a command prompt — vim/helix-style, replacing
just the status bar's hint row (the breadcrumb above it stays visible).
A popup listing every recognized command also appears, above the prompt,
for as long as it's open — no need to remember the command set. Type a
command, `Enter` to run it, `Esc` to cancel without doing anything.

- `:reindex` — manually reindexes the mounted vaults (the same reindex
  search already triggers automatically), reporting how many notes were
  indexed
- `:tags <tag1,tag2,...>` — comma-separated, matches notes with *any* of
  the listed tags (not all — there's no AND syntax yet), across *every
  mounted vault at once* (see [Tags](#tags)), each result labeled with
  its own vault. Opens a full-pane result list: `j`/`k` to move, `Enter`
  jumps to the selected note (expanding its ancestors, same as Search
  and Backlinks, and working across vaults the same way), `Esc` cancels
  back to Normal without changing your selection. If nothing matches,
  the status bar says so instead of opening an empty list.
- `:tags list` — every distinct tag across every mounted vault,
  alphabetically, with each tag's note count summed across all of them.
  `j`/`k` to move, `Enter` filters by the selected tag (same as typing
  `:tags <that-tag>` yourself, landing in the same result list as above)
  — a way to browse and pick a tag without already knowing or typing its
  exact spelling, `Esc` cancels.
- `:tags limit <vault-name>` / `:tags unlimit` — narrows `:tags`/`:tags
  list` to one named mounted vault instead of spanning all of them, until
  lifted (see [Tags](#tags)). Errors on an unknown vault name; not
  persisted across restarts.
- `:panes reset` — resets the split layout (see [Layout](#layout)) back
  to the default 40/40/20, the quickest way back after resizing since
  pane widths persist across restarts
- `:export <path>` — flattens the *selected* note's subtree to a
  Markdown or PDF file at `path`, format inferred from the extension
  (see [Exporting a subtree](#exporting-a-subtree)). Works on a
  read-only mounted vault's note too, not just the active vault's —
  exporting only reads. Refuses if `path` already exists.
- `:config unmount <show|hide>` / `:config archive <show|hide>` — shows
  or hides the `⊘`/`▦` placeholder rows for unmounted/archived vaults
  in the tree entirely (see [Layout](#layout)), for a registry with
  enough of either to feel cluttered. Persists across restarts, same as
  pane widths.
- `:tag add <tag>` / `:tag del <tag>` — adds or removes a tag on the
  *selected* note (see the tag badge row under [Layout](#layout)).
  Refuses on a read-only mounted vault's note, same as every other
  mutating command. Adding a tag that's already there, or removing one
  that isn't, reports a no-op message rather than an error.
- `:lang <en|fr|es|de>` — switches the interface language on the spot
  (the very next frame renders in the new one) and writes the choice to
  `config.toml` so it survives restarts (see
  [Configuration](#configuration)). Bare `:lang` reports the current
  language; an unknown code errors with the usage line.
- `:q` / `:quit` — quits Mycora, same as `q` `q` in Normal mode

An unrecognized command shows an error in the status bar rather than
doing nothing silently.

## Exporting a subtree

Flattens a note and its whole subtree into a single document — titles
become headings by depth (the root note is `#`, its children `##`, and
so on), and any headings already inside a note's own body are shifted
deeper by that same amount, so a note's own internal structure nests
correctly under its title instead of competing with it. No YAML
frontmatter in the output, and `[[wikilinks]]` are left as literal text
for now — rewriting ones that resolve to another note in the same
export into working Markdown anchors is a possible later improvement,
not implemented yet.

The output **format is inferred from the path's extension**: `.pdf`
renders a paginated PDF (headings, bold/italic, code blocks, and lists
all styled); anything else is written as plain Markdown. Same command
either way — nothing else about `:export`/`mycora export` changes.

From the TUI:

```
:export <path>
```

Exports the *selected* note's subtree — works on a read-only mounted
vault's note just as well as the active vault's, since exporting only
reads (see [Command palette](#command-palette)).

From the shell:

```sh
mycora export <title> <output>
```

Matches by exact title within the active vault, since a headless
invocation has no selection to work from. Errors if zero or more than
one note shares that title, rather than guessing — if a title isn't
unique, use the TUI's `:export` instead, which exports whatever's
actually selected.

Either way, **the output path must not already exist** — Mycora refuses
rather than overwriting it, since a file outside a vault has none of
Mycora's usual safety net (no trash, no undo).

## Importing an Obsidian vault

```sh
mycora import <source> <name> <path>
```

Converts an existing Obsidian vault at `<source>` into a brand new
Mycora vault registered as `<name>` at `<path>` — mounted immediately,
same as `mycora vault init`. CLI-only; there's no TUI `:import`, since
unlike export there's no "currently open" vault to import *into* — it
always creates a new one.

Obsidian has no `parent` field; its only organizational structure is the
filesystem. Folder structure becomes tree structure: a subdirectory
becomes a parent note (reusing a same-named `.md` file as that note's
own content if one exists, or an empty placeholder if not), and
everything inside it becomes children.

Per note:

- **Title** comes from the filename, not a heading inside the file.
- **Tags** carry over from frontmatter `tags:` (either `tags: single`
  or `tags: [a, b]` — Obsidian allows both). Every other frontmatter
  field is dropped; missing or unparseable frontmatter just means no
  tags, not an error.
- **Links**: `[[Title|Alias]]` and `[[Title#Heading]]` are rewritten
  down to plain `[[Title]]`, since Mycora's own wikilink resolution
  only understands that bare form — without this, aliased and
  heading-anchored links (both common in real Obsidian vaults) would
  come through broken.

`.obsidian/` and anything that isn't a `.md` file (images, canvases,
plugin data) are skipped. Refuses if `<path>` already exists and isn't
empty, same as export's refuse-on-existing-file.

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

Still a full-pane overlay rather than editing in place alongside the tree
— true split-pane editing is a separate, still-open item (see
[ROADMAP.md](./ROADMAP.md)). This is also what unblocks writing
`[[wikilinks]]` from inside the TUI instead of editing the file by hand
(see [The search index](#the-search-index)).

### Link autocompletion

Typing `[[` opens a small popup listing note titles — every note in the
vault if nothing's typed yet, narrowed to a case-insensitive prefix match
as you keep typing, spanning the active vault and every read-only
mounted one (the same scope `[[wikilinks]]` already resolve across).

- `Up`/`Down` — move the popup's selection
- `Tab` or `Enter` — accepts the selected title, replacing whatever
  you'd typed so far with the full title and a closing `]]`
- `Esc` — dismisses just the popup, leaving the rest of the edit session
  untouched (a separate `Esc` afterward still saves and exits the
  editor as usual)
- Anything else (plain typing, `Backspace`, arrow-key navigation, ...)
  keeps working normally and updates the popup to match — moving the
  cursor away from the `[[` closes it, same as if you'd never opened it

Typing `]]` yourself instead of picking a suggestion finishes the link
manually; the popup just closes once there's nothing left to match.

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

Covers renames, moves, reorders, copies, deletes, body edits, and tag
changes, for the rest of the session. Not persisted across restarts.

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
| `b` | Focus the backlinks pane (see [Backlinks](#backlinks)) |
| `Ctrl+d` / `Ctrl+u` | Scroll the body preview down / up |
| `[` / `]` | Shrink / grow the tree pane (see [Layout](#layout)) |
| `{` / `}` | Shrink / grow the backlinks pane |
| `:` | Open the command palette (see [Command palette](#command-palette)) |
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

While the `[[wikilink]]` autocomplete popup is open (see
[Link autocompletion](#link-autocompletion)), `Up`/`Down`/`Tab`/`Enter`/`Esc`
apply to the popup instead of the rows above:

| Key | Action |
|---|---|
| `↑` / `↓` | Move the popup's selection |
| `Tab` / `Enter` | Accept the selected title |
| `Esc` | Dismiss just the popup (the editor stays open) |

### Backlinks (focused)

| Key | Action |
|---|---|
| `j` / `k` / `↑` / `↓` | Move between entries |
| `Enter` | Jump to the focused entry (see [Backlinks](#backlinks)) |
| `Esc` / `b` | Return focus to the tree, keeping the current selection |

### Command

| Key | Action |
|---|---|
| *(type)* | Edit the command (see [Command palette](#command-palette)) |
| `Enter` | Run the command |
| `Esc` | Cancel without running anything |

### Tags (`:tags list`)

| Key | Action |
|---|---|
| `j` / `k` / `↑` / `↓` | Move between tags |
| `Enter` | Filter by the selected tag (opens Tag results below) |
| `Esc` | Cancel back to Normal mode |

### Tag results

| Key | Action |
|---|---|
| `j` / `k` / `↑` / `↓` | Move between results |
| `Enter` | Jump to the selected note |
| `Esc` | Cancel, keeping the current selection |
