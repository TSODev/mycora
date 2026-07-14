---
id: d67bb83f-1099-427f-b061-eae98ddd4ce9
parent: 4cbd96d7-eace-46eb-9c62-9066535f0264
order: 1
tags:
- philosophy
created: 2026-07-14T09:03:00Z
updated: 2026-07-14T09:03:00Z
---

# Single binary, no runtime dependencies

`rowdy-db` on crates.io compiles to one standalone binary via `ratatui`
+ `tokio`, no separate driver installs or runtime services beyond the
database being connected to. Keeping that promise created at least one
real constraint: both `libsql` and `sqlx`'s SQLite feature statically
bundle SQLite's own C source, and enabling both at once produces linker
errors — see [[libsql and sqlx both bundle SQLite, so only one can link]].
