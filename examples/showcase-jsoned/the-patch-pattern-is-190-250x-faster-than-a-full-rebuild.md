---
id: ee29a7f3-e3c0-40bd-a57d-d3861007ad43
parent: ac9a5dad-f9d8-4a98-80c2-14fda4395399
order: 0
tags:
- performance
- benchmark
created: 2026-07-14T09:25:00Z
updated: 2026-07-14T09:25:00Z
---

# The patch pattern is 190-250x faster than a full rebuild

Combined flatten+annotate+lint patching measured 248x faster than a
full rebuild at 1,000 items, still 190x faster at 100,000 items — see
[[The patch pattern: refresh_at, not a full rebuild]] for the mechanism
behind the number. `patch_lint` itself reads as 0.000ms on the
benchmark fixture, since it has no lint violations to begin with — the
fast no-op splice path, also the overwhelmingly common real-world case.
