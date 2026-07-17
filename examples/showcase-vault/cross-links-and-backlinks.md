---
id: 99caa33c-2179-4882-869b-a3e8728a4b22
parent: 8c33b2f7-3726-474e-9e12-8fb9ad5d434d
order: 2
tags:
- features
- links
- backlinks
- v0.5
created: 2026-07-10T09:00:00Z
updated: 2026-07-17T10:30:00Z
---

# Cross-links and backlinks

The realization of [[The mycelial layer]]: notes reference each other by
double-square-bracket title, independent of tree position.

- **Parsing** — a small hand-rolled bracket scanner extracts wikilink
  titles from a note's body, no `regex` dependency.
- **Resolution** — titles aren't required to be unique, so a wikilink
  matching more than one note fans out to a link per match; a title
  matching nothing is a **broken link** (reported, not an error); a note
  linking to its own title is skipped. See [[Fan-out ambiguous wikilinks]]
  for why fan-out specifically was chosen, and
  [[Repairing broken links]] for `mycora repair` and `:brokenlinks`,
  which actually do something about a broken one instead of only
  reporting it.
- **Cross-vault** — a wikilink can resolve to a note in any *mounted*
  vault, not just the current one — see [[Multi-vault mounting]]. This is
  the intended way to reference another vault's content, since trees
  themselves never span vaults.
- **Backlinks panel** — the right-hand pane in [[Layout]] always shows
  notes linking to the selected one, live; `b` moves keyboard focus into
  it to jump to one. Each entry also names its parent, dimmed, in
  parentheses — several notes can easily share a similarly worded title
  (more than one "Introduction" is a common one), and the parent name
  is usually enough to tell them apart before actually jumping to one.
- **Following links** — `f` is backlinks turned around: a full-pane list
  of the notes the selected note's own wikilinks resolve *to*, across
  every mounted vault. Reindexes first (unlike `b`), so a link just
  added is immediately followable.
- **Retracing your path** — following links and backlinks tends to
  wander: `Ctrl+O` jumps back to wherever you were right before your
  last search/backlinks/links/tag-results jump, repeatable to walk back
  further — see [[Navigation history]].
- **Link-count badges** — a collapsed branch shows an aggregate link
  count across its subtree, e.g. `▸ Research (12 links)`.
- **Autocompletion** — typing an opening double bracket in the body
  editor opens a popup of matching titles across every mounted vault;
  `Up`/`Down` picks, `Tab`/`Enter` accepts, `Esc` dismisses just the
  popup. See [[Full-pane body editor, save on exit]].
- **Extraction** — wikilinks aren't only typed by hand: a note's own
  table of contents can carve a heading's section into a new child
  note and drop a wikilink in its place — see
  [[Table of contents and section extraction]].
