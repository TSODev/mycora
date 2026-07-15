---
id: 2ebbc541-afce-4fea-b188-ae777ef4e310
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 10
tags:
- features
- tree
created: 2026-07-15T09:30:00Z
updated: 2026-07-15T09:30:00Z
---

# Cut, paste, and cross-vault copy

`x` marks the selected note/subtree as pending a move; `c` marks it as
pending a copy instead — a different key from [[Tree operations]]'
existing `y` (which stays exactly what it already was: an immediate
duplicate-in-place, no target picking). `p`, pressed on a destination
note, completes whichever is pending, inserting it as the destination's
last child. `Esc` cancels a pending mark without touching anything, and
the status bar shows the mark for as long as it's active — never an
invisible mode you forget you're in, the same treatment the delete
confirmation prompt already gets.

Copying works across a vault boundary: a note from any mounted
vault — [[Read-only secondary vaults]] included — can be marked with
`c` and pasted into the active one. Moving can't cross that boundary;
see [[Copying works from a read-only vault; moving doesn't]] for why
that's not an oversight. Every completed paste is one step on
[[Undo and redo]]'s stack, same as any other structural change.
