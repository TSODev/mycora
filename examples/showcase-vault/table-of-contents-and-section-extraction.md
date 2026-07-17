---
id: 55aef980-0156-4286-b5fb-9801e8df39dc
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 11
tags:
- features
created: 2026-07-17T09:00:00Z
updated: 2026-07-17T09:00:00Z
---

# Table of contents and section extraction

`t` opens a full-pane overlay listing the selected note's Markdown
headings, indented by level — `j`/`k` to move, `Esc` to cancel.

- **Jump** (`Enter`) — scrolls the [[Layout]] body preview to the
  selected heading and returns to Normal.
- **Extract** (`x`) — the more interesting one: cuts the selected
  heading's whole section out of the note and creates a **new child
  note** from it (heading text becomes the title, the rest of the
  section becomes the body), then leaves a single wikilink where the
  section used to be. See [[Cross-links and backlinks]] — this is
  another way a wikilink gets created, not just typed by hand.

Extraction is deliberately **not recursive**: a sub-heading nested
inside the extracted section stays as plain Markdown text in the new
note's body rather than being split into a note of its own. Confirmed
with the user before building it, over the alternative (cascade every
nested heading into its own note automatically) — a note that grows
too many sections shouldn't fragment itself in one keystroke into a
pile of notes nobody asked for individually. Reopen `t` on the new note
and extract a sub-heading from it yourself if you want to go a level
deeper; decomposing a long note into smaller linked ones stays a
sequence of deliberate, one-at-a-time choices.

Both halves of an extraction — the new note, and the source note's
rewritten body — undo and redo together as one step, not two: see
[[Undo and redo]] for `UndoAction::Compound`, which composes existing
single-action inverses into one for exactly this case rather than
inventing extraction-specific undo logic.
