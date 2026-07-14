---
id: 19f1e7a5-33ca-4200-b0e9-7c9f065670a4
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 9
tags:
- design-decision
- bug
- foreign-keys
created: 2026-07-14T09:30:00Z
updated: 2026-07-14T09:30:00Z
---

# FkGrid used the display label instead of the real table name

The recursive foreign-key sub-grid (see [[Foreign keys open a recursive sub-grid]]) generated its UPDATE statements using the row's *display*
label — something like `books [id=1]` — instead of the actual SQL
table name, so every edit made from inside an FK drill-down failed.
The fix separates `table_name` (the real, quotable SQL identifier) from
`display_name` (the contextual label shown in the UI), so the two can
never be confused again.
