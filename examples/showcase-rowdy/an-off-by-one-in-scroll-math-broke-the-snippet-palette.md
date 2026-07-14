---
id: 85183223-a1af-4d09-a379-0a98b41faf7b
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 3
tags:
- design-decision
- bug
created: 2026-07-14T09:24:00Z
updated: 2026-07-14T09:24:00Z
---

# An off-by-one in scroll math broke the snippet palette

The snippet palette called `.enumerate()` before `.skip()`, which meant
the index it used for display (`display_idx`) was already an absolute
position — adding the scroll offset again on top of that double-counted
it, so `real_idx == selected` was never true once the list had scrolled
past the first screen. The fix removed the redundant `real_idx`
entirely and used the already-absolute index directly.
