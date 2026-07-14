---
id: 7845e7cc-0e3b-4bca-ad8c-51ab9720053c
parent: 554adf27-7401-4b8f-916c-6559b13ccb09
order: 4
tags:
- features
- foreign-keys
created: 2026-07-14T09:15:00Z
updated: 2026-07-14T09:15:00Z
---

# Foreign key navigation

FK columns get a magenta badge in the data grid; pressing `Enter` opens
the referenced row in a recursive sub-grid, so you can follow a chain
of foreign keys as far as it goes (see [[Foreign keys open a recursive sub-grid]]). MongoDB's nested-document editing works on the same
instinct one level down — `[obj]`/`[arr:N]` fields drill into their own
sub-view rather than showing raw nested JSON inline.
