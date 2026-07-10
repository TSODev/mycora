---
id: 501863e5-3a3e-4224-9ce7-a63cefd8fc99
parent: 6238be61-b346-445d-adc0-ec88f2b9c3c7
order: 0
tags:
- philosophy
- tree
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# The tree model

Every note has exactly one parent (or is a root) — a strict
tree, navigated vim-style. This is the skeleton: it gives every note one
unambiguous "place," which a free-form graph can't.

Structural operations are all cycle-safe: [[Tree operations]] covers
move/reparent, which walks *up* from the candidate new parent to detect
whether it would create a cycle, rejecting the move if so.

The tree is deliberately not the whole model — see [[The mycelial layer]]
for the layer that sits on top of it without replacing it. See also
[[Why a strict tree]] for the reasoning behind not making this a free-form
graph instead.
