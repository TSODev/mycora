---
id: a194df7b-8c19-451d-bd5d-be4e6e7a41ae
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 5
tags:
- features
- undo-redo
created: 2026-07-10T09:00:00Z
updated: 2026-07-17T09:00:00Z
---

# Undo and redo

`u` undoes the last action, `Ctrl+R` redoes it. Covers
renames, moves, reorders, copies, deletes (see [[Tree operations]]),
body edits, and tag changes, for the rest of the session — not
persisted across restarts.

Undo/redo is built on inverses computed against the *live* tree at apply
time, not snapshots frozen when the action was originally recorded: each
undo step reads the note's current state right before mutating it and
pushes *that* onto the redo stack, rather than replaying a stored
snapshot. That's what keeps a whole chain of undo/redo correct even when
other edits happened in between one action and its eventual undo.

Editing a note's body (`e`) is also a single undo step: a whole edit
session collapses into one entry, so `u` reverts the entire session at
once rather than character by character — see
[[Full-pane body editor, save on exit]]. Same for `:tag add`/`:tag del`
(see [[Command palette]]): one undo step per command, not per tag.

Some actions are really two mutations at once — extraction's `x` (see
[[Table of contents and section extraction]]) both creates a new note
and rewrites the source note's body. Rather than inventing bespoke undo
logic for that one case, a `Compound` action wraps a list of ordinary
ones: applying it applies each in order (still against live state, same
as any other step) and collects their individual inverses into another
`Compound` for the opposite stack. One `u` or `Ctrl+R` reverses or
reapplies the whole thing, same as any single-mutation action.
