---
id: 26a3bb6d-f8c5-40ff-b8e4-6af944e3a9cf
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 4
tags:
- design-decision
- keybindings
- deferred
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Deferred: configurable keybindings

Arbitrary per-key remapping was considered and explicitly
deferred, not dropped. The current bindings are already vim-inspired and
coherent (`j`/`k`/`h`/`l`, `/` to search, `u` to undo — see [[Modes]] and
[[Status bar]] for how hints reflect them) — exactly the audience a
terminal note-taking tool draws.

Full remapping would add a real, permanent cost: a remap config schema,
conflict validation, documentation to maintain, and every future feature
having to register itself with that system — for a need that's
speculative until someone actually hits friction with the defaults.

If it's ever revisited, the plan is to prefer a small set of **named
presets** (`vim`, maybe `emacs`) over letting every key be individually
rebound — covers the realistic case (muscle memory not matching the
default) without the maintenance burden of arbitrary remapping.
