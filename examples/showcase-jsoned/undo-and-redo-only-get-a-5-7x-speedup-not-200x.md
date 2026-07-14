---
id: 6dc1428f-de72-4f70-93ba-432a13c108c5
parent: ac9a5dad-f9d8-4a98-80c2-14fda4395399
order: 1
tags:
- performance
- benchmark
created: 2026-07-14T09:26:00Z
updated: 2026-07-14T09:26:00Z
---

# Undo and redo only get a 5-7x speedup, not 200x

Not 150-250x like the rest of the patch pattern, because `undo()`/
`redo()` still do one full `JNode::clone()` for the other stack on
every call — cloning a string-heavy nested tree takes real time and
dominates the now-cheap `refresh_at` call. See [[Undo and redo still clone the whole tree]] for why, and why fixing it is scoped as later,
separate work. The 460-480ms figure itself replaced an earlier, wrong
measurement — the benchmark's own timer originally wrapped only
`refresh_at`'s portion of undo/redo (~28ms), understating the real cost
until the whole call was timed instead.
