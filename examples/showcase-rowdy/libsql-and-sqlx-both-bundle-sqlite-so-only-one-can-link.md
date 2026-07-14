---
id: 2d73fdc2-3493-4d8d-be9e-ba1ad288100f
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 0
tags:
- design-decision
- sqlite
created: 2026-07-14T09:21:00Z
updated: 2026-07-14T09:21:00Z
---

# libsql and sqlx both bundle SQLite, so only one can link

`libsql`'s `core` feature compiles `libsql-ffi`, which embeds its own
static copy of SQLite's C source — right alongside the one `sqlx`'s own
SQLite support already links in via `libsqlite3-sys`. With both
enabled, the linker sees two definitions of the same symbols
(`sqlite3_value_type` and others) and fails with `duplicate symbol`
errors, which broke `cargo install rowdy-db` entirely until 0.9.4. The
fix: only libsql's `remote` feature is enabled (`default-features =
false`) — that's all the Turso connector actually needs, and it never
touches the conflicting `core` feature at all.
