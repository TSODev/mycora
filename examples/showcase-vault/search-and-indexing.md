---
id: e32337a2-432b-409b-b10d-09bcc5e3ad7b
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 1
tags:
- features
- search
- sqlite
- v0.4
- v0.6
created: 2026-07-10T09:00:00Z
updated: 2026-07-17T09:30:00Z
---

# Search and indexing

A local SQLite index — disposable, rebuilt from the
Markdown files on demand (see [[Markdown as source of truth]]) — backs
full-text search and tag filtering.

- **Full-text search** — `/` opens a live-as-you-type overlay over
  titles + bodies, reindexing every mounted vault first so results
  reflect the tree exactly as it stands. FTS5 provides BM25-ranked
  results plus a snippet around each match, matched terms highlighted.
  Scoped to whichever vault the current selection is actually in (title
  bar names it), not always the active one — searching while browsing a
  read-only mounted vault searches *that* vault.
- **Tag filtering** — AND/OR set-filtering over tags; reachable today via
  the [[Command palette]]'s `:tags` command (OR/any-match only so far).
  Deliberately the *opposite* scoping choice from full-text search:
  `:tags`/`:tags list` span every mounted vault at once rather than just
  the current selection's — a tag is applied the same way across
  vaults, so "everything tagged X, anywhere" beats "only where I'm
  looking." `:tags list`'s counts sum across vaults; `:tags` results
  each show which vault they came from. If that gets noisy, `:tags
  limit <vault-name>` narrows both back to one named vault until `:tags
  unlimit` lifts it — not persisted across restarts, and the overlay's
  title always names the active scope.
- **Faceted search** — tag, date-range, and tree-branch facets can be
  ANDed onto a ranked query at the API level (backend only, no dedicated
  keybinding for picking facets yet).
- **Manual/watched reindex** — `mycora reindex` rebuilds every *mounted*
  vault, not just the active one — [[Multi-vault mounting]]'s read-only
  vaults get indexed right alongside it, which is what backlinks,
  link-count badges, and read-only tree navigation actually read from.
  `mycora reindex --watch` keeps all of them in sync as files change on
  disk. It also warns about every broken wikilink it finds along the
  way — see [[Repairing broken links]] for `mycora repair`, the
  headless CLI that goes further and actually fixes them.

Considered upgrading to tantivy for ranked search (originally the v0.6
goal), then reconsidered: FTS5 already does BM25 ranking and already has
a `snippet()` function, so that upgrade was deferred rather than built on
spec — see [[Disposable SQLite index]] for more on that call.
