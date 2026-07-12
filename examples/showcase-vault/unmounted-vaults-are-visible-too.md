---
id: 4a6a42c8-31bd-413c-8564-78ecf3e80c22
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 13
tags:
- design-decision
- multi-vault
created: 2026-07-12T12:00:00Z
updated: 2026-07-12T12:00:00Z
---

# Unmounted vaults are visible too

[[Read-only secondary vaults]] made every *mounted* vault besides the
active one fully navigable. An unmounted one — registered in
`config.toml` but not loaded — stayed invisible in the TUI entirely; the
only way to know it existed was `mycora vault list` from the shell, or
remembering what you'd written in the config file yourself.

Now it gets a row too: a single, unexpandable `⊘ name` placeholder
(dark gray, no fold marker — nothing is loaded for it, so there's
nothing to expand), appended after every mounted vault's section.
Selecting it shows the vault's path and the exact `mycora vault mount
<name>` command in the body preview instead of a note body, and the
breadcrumb's corner marker reads `UNMOUNTED` instead of `READ-ONLY`.
Every mutating hint dims out the same way a read-only note's does, plus
`h`/`l`/`Space` (fold) on top of that.

`App::selected` couldn't represent this — there's no `NoteId` behind an
unmounted vault, nothing was ever loaded for it — so a separate
`selected_unmounted_vault: Option<String>` field holds it instead,
mutually exclusive with `selected` through the same choke point
(`set_selected`, plus a new `set_selected_unmounted_vault` mirroring
it). Deliberately not a bigger `Selection` enum replacing `selected`
outright: that would have touched every one of the roughly 15 existing
call sites already written against a bare `Option<NoteId>`, for a
purely additive feature that didn't need the rewrite.

Auditing what every mutating command does when `self.selected` is
`None` — now reachable just by navigating onto an unmounted vault's row,
not only by deleting the very last note in an otherwise-empty vault —
turned up a real bug: pressing `o` (new sibling) with nothing selected
used to silently create a new root-level note in the *active* vault
instead of doing nothing, because `create_sibling`'s guard only checked
"is the selected note read-only," never "is anything selected at all."
Fixed to match every other mutating command's shape.
