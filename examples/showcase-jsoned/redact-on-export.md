---
id: f94e86d5-bdc6-418a-ab1c-89878d44f92c
parent: 71ca3eec-665b-4617-9dd3-702d0f4dd451
order: 11
tags:
- features
created: 2026-07-14T09:23:00Z
updated: 2026-07-14T09:23:00Z
---

# Redact on export

`--redact` on a headless export, or from the in-TUI Save-As, masks
sensitive values by key name before writing — matching on the exact key
name case-insensitively, but deliberately not fuzzy across naming
conventions ("`api_key` and `apiKey` are distinct — list both if both
appear"), a simplicity-over-cleverness tradeoff. It only ever touches
the exported copy, never the live document — so nothing needs undoing
after a redacted export.
