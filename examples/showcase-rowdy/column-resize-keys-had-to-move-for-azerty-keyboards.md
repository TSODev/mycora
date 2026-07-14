---
id: ef0114f6-d970-4d90-8f62-17ff82753008
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 10
tags:
- design-decision
created: 2026-07-14T09:31:00Z
updated: 2026-07-14T09:31:00Z
---

# Column-resize keys had to move for AZERTY keyboards

The original `[`/`]` bindings for resizing data grid columns weren't
reachable without a modifier key on AZERTY layouts, so they were
replaced with `-`/`=` instead — a small, concrete reminder that a
keyboard-only tool's bindings aren't neutral across keyboard layouts.
