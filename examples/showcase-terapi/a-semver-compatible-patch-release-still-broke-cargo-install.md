---
id: 5d0ccfc9-4d32-4a8a-bda6-2e629bf813cb
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 5
tags:
- design-decision
- dependency
created: 2026-07-14T09:24:00Z
updated: 2026-07-14T09:24:00Z
---

# A semver-compatible patch release still broke cargo install

`time` is pinned to `>=0.3, <0.3.52` after a patch release that was
supposed to be semver-compatible broke `cookie 0.18.1` transitively
through `reqwest` anyway — breaking `cargo install terapi` outright.
Kept as an explicit code comment rather than just a version pin with
no explanation, so the next person to consider bumping `time` knows
exactly what already went wrong last time: semver compatibility is a
convention, not a guarantee.
