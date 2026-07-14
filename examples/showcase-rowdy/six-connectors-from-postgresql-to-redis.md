---
id: 06d8774b-542b-4ba7-bc07-c9d3528cc172
parent: 554adf27-7401-4b8f-916c-6559b13ccb09
order: 0
tags:
- features
created: 2026-07-14T09:11:00Z
updated: 2026-07-14T09:11:00Z
---

# Six connectors, from PostgreSQL to Redis

Built in: PostgreSQL, SQLite, MySQL/MariaDB, libsql/Turso, and Redis.
Opt-in behind Cargo features: MongoDB (`--features mongodb`) and DuckDB
(`--features duckdb`) — see [[MongoDB and DuckDB are opt-in, not default]] for why they're not on by default. Each engine sits behind the
same `SqlClient`/`KvClient`/`NoSqlClient` split described in [[One interface, six database engines]].
