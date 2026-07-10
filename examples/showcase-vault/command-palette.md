---
id: e22d1f8e-0329-43b3-9648-c24ad0184361
parent: bbf99ab4-d577-4227-8285-676cb40b0d47
order: 3
tags:
- interface
- command-palette
- v0.7
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Command palette

`:` in Normal mode opens a vim/helix-style command prompt,
replacing only the [[Status bar]]'s hint row.

- `:reindex` — manually rebuilds the [[Search and indexing]] index,
  reporting how many notes were indexed
- `:tags <tag1,tag2,...>` — matches notes with *any* of the listed tags
  (OR, not AND yet), opening a full-pane result list (`j`/`k` move,
  `Enter` jumps, `Esc` cancels)
- `:q` / `:quit` — quits, same as `q` `q` in Normal mode

Both exposed commands surface backend functionality that already existed
without a keybinding of its own — see [[Search and indexing]] for the
tag-filtering and reindex machinery underneath. An unrecognized command
reports an error in the status bar rather than doing nothing silently.
