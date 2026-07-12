---
id: baac7ee6-7144-45c6-8443-160c8f053f51
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 3
tags:
- features
- multi-vault
created: 2026-07-10T09:00:00Z
updated: 2026-07-12T14:00:00Z
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

A registered vault that *isn't* mounted still shows up — as a single,
unexpandable `⊘ name` row after every mounted vault's section, since
nothing is loaded for it. Selecting it shows the vault's path and the
exact command to bring it back instead of a note body (see
[[Unmounted vaults are visible too]]).

An unmounted vault can go one step further: `mycora vault archive`
compresses its directory to a single file and deletes the original,
reclaiming the disk space entirely rather than just sitting there
unloaded — see
[[Compressing a vault trades files for one archive, deliberately]]. An
archived vault gets its own distinct `▦ name` row, not the generic
unmounted placeholder (that row's "how to mount it" message would be
wrong for something with nothing left at its path to mount) — see
[[Unmounted vaults are visible too]] for both icons side by side. Either
row category — unmounted or archived — can be hidden from the tree
entirely with `:config unmount hide`/`:config archive hide` once a
registry has enough of one to feel cluttered.

See [[Read-only secondary vaults]] for why full editing wasn't built up
front, and [[Managing vaults from the CLI]] for the `mycora vault ...`
commands that manage this registry without hand-editing `config.toml`.
