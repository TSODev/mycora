---
id: 05a89df7-faa1-4e9c-8362-5f1a14def4e6
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 8
tags:
- design-decision
- bug
created: 2026-07-14T09:29:00Z
updated: 2026-07-14T09:29:00Z
---

# Numbers were printing spurious trailing zeros

`BigDecimal::to_string()`'s internal base-10000 encoding produced
numbers like `10.6900` for what should display as `10.69`, and float
formatting used a fixed 4-decimal format even for whole numbers. The
fix strips trailing zeros while keeping a minimum of 2 decimal places,
without ever truncating a digit that actually matters.
