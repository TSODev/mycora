---
id: a194df7b-8c19-451d-bd5d-be4e6e7a41ae
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 5
tags:
- features
- undo-redo
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Undo and redo

`u` undoes the last action, `Ctrl+R` redoes it. Covers
renames, moves, reorders, copies, and deletes (see [[Tree operations]])
for the rest of the session — not persisted across restarts.

Undo/redo is built on inverses computed against the *live* tree at apply
time, not snapshots frozen when the action was originally recorded: each
undo step reads the note's current state right before mutating it and
pushes *that* onto the redo stack, rather than replaying a stored
snapshot. That's what keeps a whole chain of undo/redo correct even when
other edits happened in between one action and its eventual undo.

Editing a note's body (`e`) is also a single undo step: a whole edit
session collapses into one entry, so `u` reverts the entire session at
once rather than character by character — see [[Full-pane body editor, save on exit]].
