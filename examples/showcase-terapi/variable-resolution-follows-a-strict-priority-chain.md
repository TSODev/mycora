---
id: ec28875e-c40f-417b-b97c-4c4099de62cb
parent: d24868b2-1d5d-40e2-881b-c1e7f363bcc3
order: 2
tags:
- architecture
created: 2026-07-14T09:07:00Z
updated: 2026-07-14T09:07:00Z
---

# Variable resolution follows a strict priority chain

A `{{VAR}}` reference resolves through a fixed order: built-in
variables, then an `env_file`, then the active environment's `[env]`
table, then a campaign's `params` table-array defaults, then the current input
connector row, then a step's own `env` table, then variables extracted
from earlier steps, then runtime `-p` overrides — documented
identically in the README, USAGE.md, and CLAUDE.md, so there's one
authoritative order rather than three slightly different tellings of
it. `extract_at()`/`extract_value_at()`/`extract_segments()` implement
the dot-path language used to pull a value back out of a response
(including a `*` wildcard for arrays), which is the same mechanism
`foreach` and assertions both build on.
