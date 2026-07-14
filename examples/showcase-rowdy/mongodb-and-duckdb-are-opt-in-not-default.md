---
id: 2b5005dd-f739-456c-baca-651f0ebf6a75
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 1
tags:
- design-decision
created: 2026-07-14T09:22:00Z
updated: 2026-07-14T09:22:00Z
---

# MongoDB and DuckDB are opt-in, not default

Both connectors sit behind Cargo feature flags (`mongodb`, `duckdb`)
rather than being compiled in by default, for the same stated reason
each time: "so as not to penalize other users" who don't need them —
DuckDB in particular links a large C++ engine that's slow to compile.
Anyone who does need either engine opts in explicitly with
`--features`.
