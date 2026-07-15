---
id: d85cd778-b541-479e-811e-a5fccb18b6a7
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 16
tags:
- design-decision
- tree
created: 2026-07-15T09:30:00Z
updated: 2026-07-15T09:30:00Z
---

# Copying works from a read-only vault; moving doesn't

[[Cut, paste, and cross-vault copy]]'s `x` (move) is gated by the same
editable-vault guard as every other mutation (see
[[Guard every mutation against the wrong vault]]): a note living in one
of the [[Read-only secondary vaults]] can't be marked for a real cut,
since completing it would mean deleting it from a vault Mycora never
writes to. `c` (copy) has no such restriction — copying only ever
*reads* the source, so a note in any mounted vault, read-only included,
can be marked and pasted into the active one.

Moving a note *out of* a read-only vault stays out of scope
deliberately, not by oversight: every mutating method would need to
resolve an arbitrary target vault instead of just refusing non-active
ones, the same bigger "full multi-vault editing" lift the
[[Multi-vault mounting]] design already flags as unstarted. Cross-vault
stays copy-only until that separate piece of work is scoped.

A related fix, found by auditing every early-return in the pending-mark
flow rather than assuming it was already airtight: pasting while a
placeholder row (an unmounted or archived vault, not a real note) was
selected used to silently drop the pending mark with no feedback at
all — the one refusal path that didn't match "every refusal reported,"
unlike the read-only-target case above. Fixed by reporting it through
the same error channel as everything else, rather than letting a
pending mark vanish without a word.
