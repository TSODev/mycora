---
id: 858323d9-5725-4f46-b0ee-070144801650
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 14
tags:
- features
created: 2026-07-17T10:00:00Z
updated: 2026-07-17T10:30:00Z
---

# Navigation history

`Ctrl+O` jumps back to the note you were on just before your last
*jump* — `Enter` in [[Search and indexing]], the backlinks/outgoing-links
panels (see [[Cross-links and backlinks]]), a `:tags` result list, or
`:brokenlinks` (see [[Command palette]] and [[Repairing broken links]]).
Each press pops one more step off a stack, so pressing it repeatedly
walks back further through the path you've followed — the same
jumplist convention vim uses for `Ctrl+O`/`Ctrl+I`.

Plain `j`/`k` movement in the tree doesn't add to this history — only an
actual jump does, so `Ctrl+O` retraces the notes you followed links to,
not every row you scrolled past along the way. Deliberately not wired
into the shared selection setter every navigation path already goes
through: that would turn "walk back through your last few jumps" into
"walk back through your last few keystrokes."

No "forward" counterpart yet — `Ctrl+I` is indistinguishable from `Tab`
at the terminal level, and `Tab` already means indent. With nothing left
to go back to, `Ctrl+O` is a no-op rather than an error. Session-only,
the same shape as [[Undo and redo]]'s stacks (no inverses to compute
here, just a plain list of ids) — not persisted across restarts.
