---
id: b638d3de-00db-4686-9139-fafe5caeb066
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 12
tags:
- design-decision
- v0.9
- performance
created: 2026-07-12T11:00:00Z
updated: 2026-07-12T11:00:00Z
---

# Reindex was quadratic, one missing index fixed it

[[Roadmap]] v0.9's large-vault performance item, done the same way v0.6
resolved the tantivy question: measure first, don't guess. A new
`examples/benchmark.rs` times cold `Vault::load`, a full `mycora
reindex`, an FTS5 search, and `App::visible_rows` against synthetic
vaults from 100 to 10,000 notes — see
[[Every pane actually scrolls now]] and CLAUDE.md's own note on
`visible_rows` being "recomputed every call... revisit if it shows up
in profiling" for why that last one mattered to check specifically.
Full methodology and numbers live in `BENCHMARK.md` at the repo root,
not duplicated here.

Three of the four timed operations scaled linearly and were never a
problem — `visible_rows` included, so CLAUDE.md's own hedge turned out
to hold. `mycora reindex` didn't: **104 seconds at 10,000 notes**,
growing quadratically rather than linearly (2× the notes took 12× the
time, not 2×).

Root cause: every wikilink resolves via `WHERE title = ?1`
against the `notes` table, which had no index covering `title` — only
its `(vault_id, id)` primary key. Every single resolution was a full
table scan. *N* notes with roughly *N* wikilinks meant *O(N)* scans of
an *O(N)*-row table: *O(N²)*.

Fix was one line — `CREATE INDEX IF NOT EXISTS idx_notes_title ON
notes(title)` — plus caching a statement that was being recompiled
fresh on every single wikilink instead of once. No schema *shape*
change, so nothing to migrate: [[Disposable SQLite index]] already
means an old on-disk `index.sqlite3` just gets the new index added the
next time it's opened, same as any other `IF NOT EXISTS`.

Result: 10,000-note reindex went from 104.28s to 311.7ms — about 335×
faster, and linear like everything else now. The kind of bug that's
invisible at the vault sizes used during normal development (it doesn't
show up until thousands of notes) and exactly why this item was on the
v0.9 hardening list at all rather than assumed fine.
