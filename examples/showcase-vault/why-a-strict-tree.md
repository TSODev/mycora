---
id: a1fd8b25-f32d-4190-ac98-93014426a0a3
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 0
tags:
- design-decision
- tree
- philosophy
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Why a strict tree

[[The tree model]] gives every note exactly one parent
rather than a free-form graph, on purpose: it's what gives every note an
unambiguous "place" to be found in, the way a Zettelkasten-style flat
link graph doesn't.

The mycelial layer (see [[The mycelial layer]] and [[Cross-links and backlinks]]) is a deliberate *addition* on top of the tree, not a
replacement for it — Mycora's bet is that structure and free association
aren't actually in tension, so it doesn't have to pick one.
