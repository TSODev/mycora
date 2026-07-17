---
id: 925b8fa7-2fe7-4836-8f37-603a41d24e86
parent: d6833c8b-2dc2-4cfb-970f-c4d7537c60a0
order: 0
tags:
- roadmap
- built
created: 2026-07-10T09:00:00Z
updated: 2026-07-17T10:00:00Z
---

# Built so far

- **v0.1** — core in-memory tree model, minimal ratatui
  shell, modal input
- **v0.2** — Markdown + YAML frontmatter persistence, config file,
  self-healing on load (see [[Markdown as source of truth]])
- **v0.3** — full [[Tree operations]]: move/reparent with cycle
  detection, deep-copy, reorder, delete-to-trash, [[Undo and redo]]
- **v0.4** — the SQLite index: full-text search (FTS5), tag filtering,
  manual and watched reindex (see [[Search and indexing]])
- **v0.5** — the "mycelial" layer: wikilink parsing, backlinks
  panel, broken-link handling, cross-vault resolution, link-count badges
  (see [[Cross-links and backlinks]])
- **v0.6** — search quality: BM25 snippets/highlighting, faceted filters
  (tag/date/branch); tantivy deliberately deferred (see
  [[Disposable SQLite index]])
- **v0.7** — UX polish: full-pane body editor, theming, split-pane
  [[Layout]] with resizing, interactive backlinks focus, Markdown
  rendering in the body preview, the [[Command palette]], [[Session persistence]] — everything except configurable keybindings (see
  [[Deferred: configurable keybindings]])
- **Since v0.7** — a full `mycora vault ...` CLI for the registry:
  [[Managing vaults from the CLI]] (`add`, `init`, `rename`, `promote`,
  `mount`, `unmount`, `remove`, `archive`, `unarchive`, `list`);
  [[Read-only secondary vaults]] became fully navigable (not just
  visible), with every mutation guarded against acting on the wrong
  vault (see [[Guard every mutation against the wrong vault]]); an
  *unmounted* vault stopped being invisible too, showing up as its own
  unexpandable placeholder row with a "how to mount it" message (see
  [[Unmounted vaults are visible too]]); an unmounted vault can now be
  compressed down to a single archive file to reclaim disk space, and
  restored back, with its own distinct tree row too (see
  [[Compressing a vault trades files for one archive, deliberately]]);
  `:config unmount show/hide` and `:config archive show/hide` declutter
  the tree once a registry has enough of either
- **v0.8** — notes are never trapped in Mycora: [[Exporting a subtree]]
  flattens a note and its descendants to Markdown or PDF (see
  [[PDF export renders through a pure-Rust crate]]);
  [[Importing an Obsidian vault]] converts an existing Obsidian vault
  into a new one, mapping its folder structure onto Mycora's tree (see
  [[Folder structure becomes tree structure]]); only the optional
  stretch-goal templating hooks are left unstarted
- **Since v0.8** — `:tag add <tag>` / `:tag del <tag>` manage the
  selected note's tags directly, shown as `#tag` badges along the
  bottom of the body preview (see [[Command palette]] and [[Layout]])
- **v0.9** — done, stability before a public release:
  [[Every write to disk is atomic]] closes the crash-safety gap in
  `config.toml`/`session.toml`; an audit of `tree.rs`/`link.rs`'s test
  coverage added 19 tests for untested move/copy/delete edge cases and
  caught a real self-healing gap along the way — a self-parented note
  used to vanish from navigation instead of being healed like any other
  malformed parent (see [[Markdown as source of truth]]); a large-vault
  benchmark pass found and fixed a quadratic `mycora reindex` (see
  [[Reindex was quadratic, one missing index fixed it]]); a documentation
  audit checked USAGE.md against the actual code rather than assuming it
  was current (see [[Roadmap]]), and found (and fixed) several places it
  had quietly gone stale
- **v0.10** — done, published to crates.io: every line break typed in
  the body editor now renders as its own line in the preview, even
  without a blank line between them (see [[Layout]]'s body-preview
  note); the interface itself went multilingual — English, French,
  Spanish, German, switchable live with `:lang <en|fr|es|de>` and
  persisted to `config.toml` (see
  [[The interface speaks four languages]]); every mounted vault gets a
  centered, background-colored name header in the tree pane (see
  [[Layout]]); the body editor now offers wikilink autocompletion as you
  type (see [[Cross-links and backlinks]]) — the last of the two
  long-deferred headline items from early on, open since v0.5; `f`
  follows a note's outgoing wikilinks, the backlinks pane's exact mirror
  image (see [[Cross-links and backlinks]]); and the status bar got two
  fixes at once — Normal mode's hint row, grown to 233 characters over
  several versions, is now a short curated set plus a `?` full-pane
  reference for everything else, and the breadcrumb row gained a
  centered "last modified" timestamp, shown only when there's room
  (see [[Status bar]])
- **Since v0.10** — renaming a note now renames its underlying `.md`
  file too, instead of leaving it stuck with whatever name it got on
  first save (often "New note"); `mycora vault sync-filenames <name>`
  retroactively fixes notes that already drifted before this existed
  (see [[Markdown as source of truth]]); and the SQLite index opens in
  WAL mode with a real busy timeout, a cheap step toward the still-open
  concurrent-write-safety question (see [[Disposable SQLite index]])
- **v0.11** — [[Cut, paste, and cross-vault copy]]: `x` marks a
  note/subtree to move, `c` marks it to copy (copying alone works from a
  read-only mounted vault too — see
  [[Copying works from a read-only vault; moving doesn't]]), `p` on a
  destination completes whichever is pending as its last child, `Esc`
  cancels a pending mark
- **Since v0.11** — [[Attaching files to a note]] copies a file into
  `attachments/` and links it from the cursor, `Ctrl+A` while editing a
  body (see [[Attachments are copied and linked, never rendered]] for
  why nothing renders inline); three more showcase vaults joined this
  one, documenting sibling projects [[Rowdy]], [[Terapi]], and
  [[Jsoned]] the same way this vault documents Mycora itself (see
  [[Other projects]]); `PUBLISH.md` written up as a real release
  checklist instead of living only in memory from one release to the
  next
- **v0.13** — published to crates.io: the body preview pane learned to
  render Markdown tables as a bordered grid instead of literal `|`
  text, columns sized by actual terminal display width rather than
  `char` count so emoji/CJK content keeps its borders aligned instead
  of drifting (see [[Layout]]); `t` opens a
  [[Table of contents and section extraction]] overlay over a note's
  headings, `Enter` to jump, `x` to extract a heading's section into a
  new linked child note as a single undoable step (see
  [[Undo and redo]]); and `:import <path>` pulls a single external
  Markdown file into the open vault as a new child note, sharing its
  parser with [[Importing an Obsidian vault]]'s bulk import instead of
  duplicating it (see [[Importing a single Markdown file]])
- **Since v0.13** — `mycora repair` reports (and, with `--create-stubs`/
  `--apply`, fixes) broken wikilinks across every mounted vault, in
  tiers from safe to destructive (see [[Repairing broken links]]);
  backlinks pane entries now name their parent, dimmed, so
  similarly-titled notes stay distinguishable before you jump to one
  (see [[Cross-links and backlinks]]); and `Ctrl+O` jumps back through
  your last few search/backlinks/links/tag-results jumps, vim-jumplist
  style (see [[Navigation history]])
