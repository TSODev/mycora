---
id: dd59c74f-04cf-4e89-bcf3-ebaa5a209e13
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 6
tags:
- design-decision
created: 2026-07-14T09:27:00Z
updated: 2026-07-14T09:27:00Z
---

# A known DuckDB engine limitation makes some updates falsely fail

DuckDB 1.x itself rejects `UPDATE` statements on complex-typed columns
(`VARCHAR[]`, `STRUCT`) when the row has an incoming foreign key
reference, reporting a false constraint violation — this is documented
as a known limitation of the underlying engine, not a Rowdy bug, with
the workaround being to run the update through the SQL Editor directly
or drop the foreign key from the seed schema.
