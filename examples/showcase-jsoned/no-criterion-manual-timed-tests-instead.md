---
id: 04900191-2607-4dc6-b700-314262fa9927
parent: cc955771-76eb-4c7e-af57-38e84c4d4224
order: 4
tags:
- design-decision
- performance
created: 2026-07-14T09:32:00Z
updated: 2026-07-14T09:32:00Z
---

# No criterion: manual timed tests instead

jsoned is a binary-only crate — no `lib.rs` — and several of the
functions most worth benchmarking (`push_undo`, `undo`, `refresh_at`)
are private, so an external `criterion` bench file would only ever be
able to see the crate's `pub` surface. Rather than splitting into a
lib+bin just to unlock a proper benchmarking harness ("not worth it for
a project this size"), `#[cfg(test)] #[test] #[ignore]` functions with
manual `Instant` timing take its place — see [[Performance]] for what
they measured.
