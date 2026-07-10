---
id: cfff6ed6-c070-4376-a2ff-f1416653a3d2
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 5
tags:
- design-decision
- search
- sqlite
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Disposable SQLite index

"The index is disposable" is one of Mycora's core
principles (see [[Markdown as source of truth]]): the SQLite database
behind [[Search and indexing]] is entirely derived from the
Markdown files and can be rebuilt from scratch at any time — you can
always delete it and regenerate it with `mycora reindex`.

This is also why an internal schema change (e.g. the `links` table
gaining separate source/target vault columns to support cross-vault
resolution) doesn't need a real migration: on open, the old table shape
is detected and simply dropped and recreated, since nothing in it is
data that can't be regenerated for free.

The same instinct shaped the tantivy-vs-FTS5 call: rather than adding a
second full-text engine on spec to hit the v0.6 "ranked search" goal,
FTS5's already-built-in BM25 ranking and snippet support were used
directly, since that goal was already met without new machinery. Tantivy
stays a deferred option, revisited only if a concrete gap in FTS5 shows
up (typo tolerance, ranking quality at real scale).
