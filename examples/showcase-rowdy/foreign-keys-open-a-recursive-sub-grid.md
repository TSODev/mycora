---
id: c5ce8303-2356-4441-a227-438fd2b1e002
parent: 6d5eb76b-9b62-4c58-96e3-90c1e30f7fc7
order: 2
tags:
- architecture
- foreign-keys
created: 2026-07-14T09:07:00Z
updated: 2026-07-14T09:07:00Z
---

# Foreign keys open a recursive sub-grid

Foreign key columns are detected per-connector at schema-introspection
time (`ColumnSchema.fk`, populated from `pg_catalog`,
`information_schema`, or SQLite's `PRAGMA`) and shown with a magenta
badge in the data grid. Pressing `Enter` on one opens a recursive
sub-grid (`FkGrid` state, backed by an `fk_history` stack), so following
a chain of foreign keys is just more of the same screen, not a
special-cased view — see [[Foreign key navigation]] for the feature
itself, and [[FkGrid used the display label instead of the real table name]] for a bug this recursion once caused.
