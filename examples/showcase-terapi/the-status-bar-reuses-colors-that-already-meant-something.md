---
id: ce33828a-c71e-4de3-aae8-e891f676a5b6
parent: 2423e022-ff5b-4605-b555-48b7500dd11f
order: 10
tags:
- design-decision
created: 2026-07-14T09:29:00Z
updated: 2026-07-14T09:29:00Z
---

# The status bar reuses colors that already meant something

A status bar redesign deliberately reused existing color conventions
rather than inventing new ones: status codes keep the same
2xx/3xx/4xx/5xx scheme already used elsewhere, and elapsed-time
coloring reuses the exact thresholds already established in the HTTP
view's Diagnostics section — "rather than inventing a second
convention" for what's functionally the same information shown in a
new place.
