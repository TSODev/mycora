---
id: 4c5a908b-fa28-40f7-b1ad-13a9286eb293
parent: 554adf27-7401-4b8f-916c-6559b13ccb09
order: 8
tags:
- features
created: 2026-07-14T09:19:00Z
updated: 2026-07-14T09:19:00Z
---

# Exporting to CSV and JSON, with foreign-key resolution

CSV export follows RFC 4180; JSON export has two modes — a plain dump,
or one that resolves foreign keys inline as `<col>__ref` objects,
recursively up to 3 levels deep with cycle detection, so an exported
row can carry its related data with it instead of just the raw foreign
key value.
