---
id: 161747d1-57ee-4f44-9e14-a9b5db74faaa
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 0
tags:
- design-decision
- bug
created: 2026-07-14T09:19:00Z
updated: 2026-07-14T09:19:00Z
---

# A dead connection pool bug only showed up after a large request

A small request sent right after a large one on the same host used to
hang forever — order-dependent, and hard to reproduce until it was:
"consistent with reqwest handing the next request a pooled keep-alive
connection that just carried the large transfer and turned out to be
dead." The fix sets `.pool_max_idle_per_host(0)` on all four of
terapi's `reqwest` clients, forcing a fresh connection per request —
judged an acceptable cost since "the small added latency (a fresh
handshake) is a non-issue for an interactive API client."
