---
id: 57620b1d-4088-4679-9e17-bd580ec85dcb
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 2
tags:
- design-decision
- multi-vault
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T22:00:00Z
---

# Read-only secondary vaults

[[Multi-vault mounting]]'s first pass makes every vault
except the active one read-only in the TUI, rather than building full
multi-vault editing from the start.

Full editing would need every mutating operation in the app to first
resolve which vault a given note actually belongs to — a real change
touching a large number of methods, still not attempted. What *did* land
later the same day: read-only vaults are fully navigable, not just
visible — `j`/`k` move into them, branches expand/collapse, the body
preview and [[Cross-links and backlinks]] panel both work for whatever's
selected in any mounted vault, and jumping to a backlink can land in any
of them. Every edit key still refuses with a clear "this vault is
read-only" instead of silently doing nothing or, worse, mutating the
active vault by mistake — see
[[Guard every mutation against the wrong vault]].

Link-count badges needed none of this — they worked for read-only vaults
from the start, since they only need a vault id, not a selectable tree —
which is what originally proved the shared [[Search and indexing]] index
works correctly across mounted vaults, before navigation caught up.
