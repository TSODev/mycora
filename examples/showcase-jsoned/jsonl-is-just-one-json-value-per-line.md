---
id: 2ce3bb57-74fc-4076-8965-bad559098e09
parent: cc955771-76eb-4c7e-af57-38e84c4d4224
order: 0
tags:
- design-decision
created: 2026-07-14T09:28:00Z
updated: 2026-07-14T09:28:00Z
---

# JSONL is just one JSON value per line

JSONL is modeled as one JSON value per line, equivalent to a single
JSON array — once parsed, there's no special-casing anywhere else in
the tree, flatten, annotate, lint, or patch machinery; it's just
another `JNode` root like any array would be. Export mirrors the same
root-handling rule CSV export uses (an array root becomes one line per
element, anything else becomes a single line) — except unlike CSV,
JSONL doesn't require object-shaped rows, so there's no error case to
handle on the way out.
