---
id: c665718f-7868-4312-9591-5551b4fdad47
parent: e7c5cb1f-340e-4b2a-b237-5aaa8475bca4
order: 2
tags:
- design-decision
- bug
created: 2026-07-14T09:23:00Z
updated: 2026-07-14T09:23:00Z
---

# A missing schemas_supported flag hung the schema panel forever

`spawn_load_all_schemas()` never reset its `schemas_loading` flag for
non-SQL connectors (Redis, MongoDB before schema support), so the
schema panel was stuck showing "Loading schema…" indefinitely, and
pressing `r` for the ERD view did nothing. The fix added a
`schemas_supported: bool` field, true by default and set false for
KV/NoSQL connectors, so the panel can immediately show "Schema not
available for this connector type" instead of waiting on a load that
was never coming.
