---
id: 33592da2-7529-4346-816a-2429e486ef7d
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 0
tags:
- features
- tree
- undo-redo
created: 2026-07-10T09:00:00Z
updated: 2026-07-13T20:00:00Z
---

# Tree operations

All the structural operations on the [[The tree model]]:

- **Create** — `a` for a new child, `o` for a new sibling
- **Rename** — `i`, prefilled with the current title; the underlying
  `.md` file's name is kept in sync too (see
  [[Markdown as source of truth]]) — notes created before that existed
  can be caught up with `mycora vault sync-filenames <name>`
- **Move / reparent** — `Tab`/`Shift+Tab` indent/outdent; internally
  cycle-safe, walking *up* from the candidate new parent to reject any
  move that would create a loop
- **Reorder** — `K`/`J` move a note up/down among its siblings
- **Copy** — `y` deep-copies a subtree: fresh ids and timestamps for
  every node, no shared identity with the original (see
  [[Fan-out ambiguous wikilinks]] for why a link-only copy waits on the
  cross-link layer instead)
- **Delete** — `d` asks for confirmation, then moves the whole subtree to
  `.trash/` rather than erasing it outright; trash is never auto-emptied
  or auto-scanned

All of the above are covered by [[Undo and redo]] for the rest of the
session.
