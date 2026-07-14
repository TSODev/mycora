---
id: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
parent: 16cc5a85-e964-4b86-8a4a-6aec474e8530
order: 3
tags:
- design-decision
created: 2026-07-14T09:20:00Z
updated: 2026-07-14T09:20:00Z
---

# Design decisions

Specific choices made along the way, mostly surfaced by real bugs
rather than decided upfront — the "why," not just the "what."

- [[libsql and sqlx both bundle SQLite, so only one can link]]
- [[MongoDB and DuckDB are opt-in, not default]]
- [[A missing schemas_supported flag hung the schema panel forever]]
- [[An off-by-one in scroll math broke the snippet palette]]
- [[The data grid remembers your row after a filter or sort]]
- [[A keyring library silently lied about writing the credential]]
- [[A known DuckDB engine limitation makes some updates falsely fail]]
- [[Disconnect hooks run fire-and-forget, except on the way out]]
- [[Numbers were printing spurious trailing zeros]]
- [[FkGrid used the display label instead of the real table name]]
- [[Column-resize keys had to move for AZERTY keyboards]]
- [[Dead code marked, not deleted, when it's reserved for later]]
- [[Trimming build artifacts shrank the published crate by 90 percent]]

See [[Roadmap]] for the versions each of these landed in.
