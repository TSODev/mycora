---
id: baac7ee6-7144-45c6-8443-160c8f053f51
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 3
tags:
- features
- multi-vault
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T22:00:00Z
---

# Multi-vault mounting

Mycora maintains a **registry** of named vaults in
`config.toml`; only the ones marked `mounted` load at startup. This is a
registry/mount split, not a tree merge — each mounted vault keeps its own
independent tree with its own roots, deliberately not flattened into one
shared super-tree (that would need either a synthetic super-root or
letting moves reparent across vaults, and a cross-vault move doesn't fit
how a `Vault` owns one on-disk directory).

Only one vault — the **active** one — is editable in the TUI; every other
mounted vault is shown read-only, stacked below the active vault's tree
with a dimmed separator. Read-only doesn't mean hidden, though: they're
fully navigable — `j`/`k` moves into them, branches expand and collapse,
and the body preview and backlinks panel both work for whatever's
selected. Read-only vaults are still indexed and still contribute to
link-count badges and cross-vault link resolution (see
[[Cross-links and backlinks]]); the one thing that never works on them is
editing — every mutating key refuses with "this vault is read-only" (see
[[Guard every mutation against the wrong vault]]). `/` full-text search
stays scoped to the active vault's notes only, for now.

See [[Read-only secondary vaults]] for why full editing wasn't built up
front, and [[Managing vaults from the CLI]] for the `mycora vault ...`
commands that manage this registry without hand-editing `config.toml`.
