---
id: e2781d98-b57b-4090-aa77-aa120f39dabd
parent: 554adf27-7401-4b8f-916c-6559b13ccb09
order: 7
tags:
- features
created: 2026-07-14T09:18:00Z
updated: 2026-07-14T09:18:00Z
---

# Schema introspection and the ERD graph view

Primary keys, foreign keys, and column types are introspected
per-engine (`pg_catalog`, `information_schema`, or SQLite's `PRAGMA`)
and surfaced two ways: a schema panel in the table list (PK/FK badges,
outgoing and incoming foreign keys), and a full ERD graph (`r`) laying
tables out in a star pattern with bent ASCII arrows between them,
navigable with `j`/`k`. See [[A missing schemas_supported flag hung the schema panel forever]] for a bug this feature had for non-SQL
connectors.
