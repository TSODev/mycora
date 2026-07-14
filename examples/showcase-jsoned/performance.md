---
id: ac9a5dad-f9d8-4a98-80c2-14fda4395399
parent: 599f041a-3fa6-4fdd-b145-8fb8f7e8a8ef
order: 3
tags:
- performance
- benchmark
created: 2026-07-14T09:24:00Z
updated: 2026-07-14T09:24:00Z
---

# Performance

No `criterion` benchmarks — jsoned is a binary-only crate (no
`lib.rs`), and several of the functions worth measuring (`push_undo`,
`undo`, `refresh_at`) are private, so an external bench file would only
ever see the `pub` surface. Manual `#[cfg(test)] #[test] #[ignore]`
functions with `Instant` timing take their place instead, three repeats
reporting the minimum — see [[No criterion: manual timed tests instead]].

- [[The patch pattern is 190-250x faster than a full rebuild]]
- [[Undo and redo only get a 5-7x speedup, not 200x]]

Fixtures are synthetic: arrays of N objects (1k/10k/50k/100k), 8 fields
each, always editing the last item — conservative numbers, not
best-case, since that's the worst case for the position-scan the patch
pattern depends on.
