---
id: a38e4705-680f-43b6-9790-50e55cef2c86
parent: 71ca3eec-665b-4617-9dd3-702d0f4dd451
order: 0
tags:
- features
created: 2026-07-14T09:12:00Z
updated: 2026-07-14T09:12:00Z
---

# Three-panel layout

Source (the annotated JSON itself), Explorer (a key/type/value table,
the primary place to navigate and edit from), and Detail (a preview of
the selected node) — three views onto the same `JNode` tree, always in
sync since they're all derived from it via [[The patch pattern: refresh_at, not a full rebuild]] rather than kept separately.
