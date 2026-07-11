---
id: e22d1f8e-0329-43b3-9648-c24ad0184361
parent: bbf99ab4-d577-4227-8285-676cb40b0d47
order: 3
tags:
- interface
- command-palette
- v0.7
created: 2026-07-10T09:00:00Z
updated: 2026-07-11T09:00:00Z
---

# Command palette

`:` in Normal mode opens a vim/helix-style command prompt,
replacing only the [[Status bar]]'s hint row. A popup listing every
command also appears above the prompt for as long as it's open.

- `:reindex` — manually rebuilds the [[Search and indexing]] index,
  reporting how many notes were indexed
- `:tags <tag1,tag2,...>` — matches notes with *any* of the listed tags
  (OR, not AND yet), opening a full-pane result list (`j`/`k` move,
  `Enter` jumps, `Esc` cancels)
- `:tags list` — every distinct tag, alphabetically with note counts;
  `Enter` on one filters by it, landing in the same result list as
  above — pick a tag without already knowing or typing its exact
  spelling. Live autocompletion while typing `:tags <partial>` was
  considered too, then deferred — more implementation work for a need
  this already covers in practice.
- `:panes reset` — resets the [[Layout]] back to 40/40/20, the way back
  after resizing now that widths persist across restarts
- `:export <path>` — flattens the *selected* note's subtree to Markdown
  at `path`, refusing if it already exists — see
  [[Exporting a subtree]]
- `:q` / `:quit` — quits, same as `q` `q` in Normal mode

Every exposed command surfaces backend functionality or a real gap that
had no keybinding of its own — see [[Search and indexing]] for the
tag-filtering and reindex machinery underneath `:reindex`/`:tags`. A
`:search` command was considered too, then skipped: `/` already has a
direct keybinding, so it would just duplicate an existing entry point
rather than adding anything. An unrecognized command reports an error in
the status bar rather than doing nothing silently.
