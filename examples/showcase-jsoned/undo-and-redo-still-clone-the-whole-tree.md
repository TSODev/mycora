---
id: b53e3da4-9886-4ce8-91f4-482fa1e894e4
parent: d74fbdfa-cfbf-4827-9a42-d05bae30e309
order: 2
tags:
- architecture
- performance
created: 2026-07-14T09:07:00Z
updated: 2026-07-14T09:07:00Z
---

# Undo and redo still clone the whole tree

`UndoEntry { target: JPath, root: JNode }` reuses `refresh_at` when a
history entry is popped, but still does one full `JNode::clone()` per
push — explicitly flagged as a known, documented, not-yet-optimized
cost, since the patch pattern used everywhere else doesn't extend to
undo/redo's own storage. On a roughly 875,000-row file, a single value
edit runs in 28-48ms, but an undo takes 460-480ms — almost entirely the
clone, not the patch. A real fix (structural sharing via `Rc`, or
diff-based history instead of full snapshots) is named explicitly as a
larger, separate project, not part of this work.
