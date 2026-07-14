---
id: 0b5113f7-d2ef-4191-93f4-ce077580ceb6
parent: 71ca3eec-665b-4617-9dd3-702d0f4dd451
order: 5
tags:
- features
created: 2026-07-14T09:17:00Z
updated: 2026-07-14T09:17:00Z
---

# Structural lint

Automatic checks run on load and after every edit — empty keys,
excessive nesting depth beyond `MAX_DEPTH` (20) — with warnings
highlighted in orange and `Tab`/`Shift+Tab` to jump between them.
Fixing a flagged node clears its warning instantly, patched in place
the same way everything else is — see [[The patch pattern: refresh_at, not a full rebuild]] for the one case (a brand new warning in a
previously clean subtree) this patching still has to fall back to a
full re-lint for.
