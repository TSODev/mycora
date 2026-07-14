---
id: ffe148a1-7234-4d98-b543-d27373e78b58
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 4
tags:
- design-decision
created: 2026-07-14T09:25:00Z
updated: 2026-07-14T09:25:00Z
---

# The data grid remembers your row after a filter or sort

A new `preserved_row: Option<usize>` field on the data grid screen is
saved whenever data resets and restored (clamped to the new row count)
whenever a fresh result set arrives — so applying a filter, changing
the sort, or saving an edit no longer jumps you back to row one.
Opening a different table still starts at row zero, as expected; this
only preserves position across a reload of the *same* result set.
