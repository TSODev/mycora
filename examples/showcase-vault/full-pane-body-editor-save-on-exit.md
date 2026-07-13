---
id: 58cbadfc-94ce-44c6-a3e4-d06c389a14b1
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 3
tags:
- design-decision
- editor
- v0.7
created: 2026-07-10T09:00:00Z
updated: 2026-07-13T16:00:00Z
---

# Full-pane body editor, save on exit

`e` opens a note's body in a **full-pane** overlay, built
on the `ratatui-textarea` crate rather than a hand-rolled multi-line
editor — the kind of easy-to-get-wrong functionality (UTF-8 cursor
movement, line editing) that's worth an established dependency for.

`Esc` saves and exits, with no separate discard-without-saving path — a
whole edit session is just one `u`-undoable step if you want to back out
afterward (see [[Undo and redo]]), consistent with the rest of the app's
"no explicit save" philosophy (see [[Markdown as source of truth]]). A
no-op edit — nothing actually changed — skips both the disk write and the
undo entry.

Deliberately **not** the split-pane [[Layout]]: true in-place
split-pane editing was kept open as a separate, not-yet-built item on
purpose, rather than being backed into as a side effect of building this
editor.

This is also what unblocked [[Cross-links and backlinks]]'s wikilink
autocompletion — there was nowhere to type `[[` in the TUI at all until
this editor existed, even once autocompleting it was worth building.
