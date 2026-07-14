---
id: 528e3cb8-26e4-4e04-9019-f65f53205df8
parent: 554adf27-7401-4b8f-916c-6559b13ccb09
order: 6
tags:
- features
created: 2026-07-14T09:17:00Z
updated: 2026-07-14T09:17:00Z
---

# Record editing with format validation

The Edit Record screen shows `[PK]`/`[→FK]` badges next to the relevant
fields, a live preview of the exact SQL statement it's about to run,
and a confirmation modal before it runs. Fields are validated by type
before you can save — DATE/TIME/TIMESTAMP/UUID/JSON/INET/CIDR all get
real format checks (highlighted red, `Ctrl+S` blocked) rather than only
failing at the database. MongoDB's document editing shares the same
screen shape but with `Enter` for replace, `a` to insert, `D` to
delete, and `i` for a raw JSON edit.
