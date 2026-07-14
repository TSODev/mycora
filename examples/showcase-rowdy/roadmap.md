---
id: f0153899-246a-4d80-93fc-fd189973329a
parent: 16cc5a85-e964-4b86-8a4a-6aec474e8530
order: 4
tags:
- roadmap
created: 2026-07-14T09:34:00Z
updated: 2026-07-14T09:34:00Z
---

# Roadmap

Rowdy doesn't keep a separate forward-looking roadmap document the way
Mycora does — this note is just the released-version history from
`CHANGELOG.md`, read as a timeline.

Latest release: **0.9.4** (the libsql/sqlite linker fix, see [[libsql and sqlx both bundle SQLite, so only one can link]]). Before that: 0.9.3
(SQL snippets), 0.9.2 (crate packaging cleanup + the schema panel fix),
0.9.1 (the `Ctrl+F` key conflict fix), 0.9.0 (full-text search + cursor
persistence), 0.8.5 (117 integration tests across every connector),
0.8.4 (multi-tab sessions + auto-reconnect), 0.8.3 (keyring
encryption), 0.8.2 (the DuckDB connector), 0.8.1 (MongoDB CRUD polish),
0.8.0 (MongoDB document editing, nested navigation, the ERD view, the
schema panel, connect hooks) — down through 0.1.0's initial connector
infrastructure.

The one forward-looking note left in the project's own docs isn't a
feature at all: a warning, in `CLAUDE.md`, to keep `libsql` at
`default-features = false` — re-enabling its `core` feature would bring
back the exact linker conflict [[libsql and sqlx both bundle SQLite, so only one can link]] describes.
