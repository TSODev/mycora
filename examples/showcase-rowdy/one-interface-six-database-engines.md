---
id: b2ba4e96-1637-48c5-a43a-53152fb3922b
parent: 4cbd96d7-eace-46eb-9c62-9066535f0264
order: 0
tags:
- philosophy
- architecture
created: 2026-07-14T09:02:00Z
updated: 2026-07-14T09:02:00Z
---

# One interface, six database engines

Three traits in `src/db/traits/` do all the work of hiding six engines
behind one app: `SqlClient` (PostgreSQL, SQLite, MySQL, libsql/Turso,
DuckDB — `connect`, `execute`, `fetch_all`, `get_table_objects`,
`get_schema`), `KvClient` (Redis), and `NoSqlClient` (MongoDB). Concrete
implementations live one file per engine under `src/db/connectors/`, and
`connectors/mod.rs`'s `connect_sql`/`connect_kv`/`connect_nosql` are the
single dispatch point mapping a `db_type` string to a boxed trait
object.

Adding a new engine is mechanical rather than architectural: implement
the right trait, register it in the matching `connect_*` function, and
add it to `DB_TYPES` in the connection screen. MongoDB and DuckDB are
gated behind `#[cfg(feature = "mongodb"/"duckdb")]` throughout, rather
than always compiled in — see [[MongoDB and DuckDB are opt-in, not default]].
