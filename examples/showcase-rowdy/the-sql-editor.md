---
id: eeb2166c-ced5-47ce-9631-4307861edf2a
parent: 554adf27-7401-4b8f-916c-6559b13ccb09
order: 5
tags:
- features
created: 2026-07-14T09:16:00Z
updated: 2026-07-14T09:16:00Z
---

# The SQL editor

A `tui-textarea`-based multi-line editor (`F5`/`Ctrl+Enter` to run,
`Ctrl+Q` to exit) that splits on `;` to execute multiple statements in
one go, reporting errors per statement rather than aborting the whole
batch on the first failure. Autocomplete (`Tab`) covers table names,
column names, and roughly 80 SQL keywords; query history
(`Alt+↑`/`Alt+↓`) persists up to 200 deduplicated entries; a snippet
palette (`Ctrl+P` to open, `Ctrl+S` to save) persists reusable queries
in `snippets.toml`. `F4` opens a `SELECT` result in the same full,
read-only data grid used everywhere else, rather than a separate
results view.
