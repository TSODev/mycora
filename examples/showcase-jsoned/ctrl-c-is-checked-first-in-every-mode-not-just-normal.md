---
id: a097fcd2-355c-4f12-837f-5a816ea8fdf8
parent: cc955771-76eb-4c7e-af57-38e84c4d4224
order: 3
tags:
- design-decision
- bug
created: 2026-07-14T09:31:00Z
updated: 2026-07-14T09:31:00Z
---

# Ctrl+C is checked first in every mode, not just Normal

`Ctrl+C` used to only do something in Normal mode — pressing it during
an open edit, a Save-As dialog, or a plugin prompt did nothing at all.
Fixed so every mode's key handler checks for it first, before any
mode-specific dispatch, matching the same always-available emergency
quit reasoning Mycora's own event loop uses for the same key.
