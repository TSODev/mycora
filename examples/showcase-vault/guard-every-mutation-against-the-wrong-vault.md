---
id: 85bff86e-c649-44b6-a3cd-1f805b91f11e
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 7
tags:
- design-decision
- multi-vault
created: 2026-07-10T22:00:00Z
updated: 2026-07-10T22:00:00Z
---

# Guard every mutation against the wrong vault

Making [[Read-only secondary vaults]] fully navigable — selectable, not
just visible — meant the selected note could, for the first time, live
outside the active vault's tree. Auditing every command that acts on
"the selected note" before writing any of that navigation code turned up
two real bugs it would otherwise have triggered or left silently broken:

- **Creating a child or sibling note had no check at all.** Given a
  selected id from a read-only vault, the active vault's tree would have
  happily created a brand-new note wrongly parented under an id it
  doesn't actually own — a stray note appearing in *your* vault, caused
  by browsing someone else's. Not a crash; worse, silent data you didn't
  ask for.
- **The breadcrumb assumed the selection was always in the active
  vault.** Once it wasn't, the breadcrumb would have shown the active
  vault's name next to an empty path — technically not wrong about
  *which* vault is active, just actively misleading about *where you
  actually are*.

Every other mutating command (copy, indent/outdent, reorder, rename, body
edit) already happened to no-op safely on a foreign id, via existing
`None`-checks that were never written with this case in mind — but
silently, with no feedback distinguishing "nothing to do" from "that's
not allowed here."

The fix is one rule, applied uniformly rather than patched command by
command: every mutating action checks whether the selected note belongs
to the active vault *first*, before doing anything else, and reports
"this vault is read-only" if not — the same message whether you pressed
`a`, `d`, `i`, `e`, `y`, or a reorder key. One rule, one message,
everywhere, rather than nine slightly different behaviors depending on
which existing safety net a given command happened to already have.
