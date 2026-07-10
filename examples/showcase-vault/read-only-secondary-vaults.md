---
id: 57620b1d-4088-4679-9e17-bd580ec85dcb
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 2
tags:
- design-decision
- multi-vault
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Read-only secondary vaults

[[Multi-vault mounting]]'s first pass makes every vault
except the active one read-only in the TUI, rather than building full
multi-vault editing from the start.

Full editing would need every mutating operation in the app to first
resolve which vault a given note actually belongs to — a real change
touching a large number of methods. Read-only-first ships the part that
matters immediately (seeing and cross-linking to another vault's notes)
without taking on that cost speculatively. Search and the backlinks panel
are similarly scoped to the editable vault only, since a jump-to-result
has nowhere to land in a vault the tree can't select into.

Link-count badges are the exception — they work for read-only vaults too,
since they only need a vault id, not a selectable tree — which is what
actually proves the shared [[Search and indexing]] index works correctly
across mounted vaults.
