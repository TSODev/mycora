---
id: 2a676648-5751-4c49-bfc6-b925ff8395ef
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 7
tags:
- features
- export
- v0.8
created: 2026-07-11T09:00:00Z
updated: 2026-07-11T09:00:00Z
---

# Exporting a subtree

The first [[Roadmap]] v0.8 item: flattening a note and its whole
subtree into a single, portable Markdown document — notes should never
be trapped in Mycora.

- Titles become headings by depth: the exported root note is `#`, its
  children `##`, grandchildren `###`, and so on.
- Any headings already inside a note's own body get shifted deeper by
  that same amount, so a note's own internal structure nests correctly
  under its title rather than competing with it.
- No YAML frontmatter in the output, and wikilinks are left as literal
  text — the same way the [[Layout]] body preview already renders them,
  not something the export tries to resolve yet.

Two ways to trigger it, both landed together:

- `:export <path>` in the [[Command palette]] — exports the *selected*
  note's subtree. Works on a read-only mounted vault's note too (see
  [[Multi-vault mounting]]), since exporting only reads.
- `mycora export <title> <output>` from the shell — matches by exact
  title within the active vault, since a headless invocation has no
  selection to work from. Errors on zero or multiple matches rather
  than guessing — the same instinct as
  [[Fan-out ambiguous wikilinks]] and `vault promote`'s refusal — and
  points at `:export` for the disambiguate-by-selection case.

Either path **refuses if the output file already exists** rather than
overwriting it: a path outside a vault has none of Mycora's usual
safety net ([[Undo and redo]], the trash) to fall back on if it went
wrong.

Exporting to PDF is next on the [[Roadmap]] — most likely built on top
of this Markdown export (flatten first, then render that), with the
rendering approach and command surface still open questions.
