---
id: f7f47822-6083-48be-8ab1-789369357de1
parent: bbf99ab4-d577-4227-8285-676cb40b0d47
order: 1
tags:
- interface
- modes
created: 2026-07-10T09:00:00Z
updated: 2026-07-17T09:00:00Z
---

# Modes

Mycora is modal, vim-style. The current mode drives both
what keys do and what the [[Status bar]]'s hint row shows.

- **Normal** — the default: navigate, create, rename, delete, undo, open
  [[Search and indexing]], focus [[Cross-links and backlinks]], enter the
  [[Command palette]]
- **Insert** — naming or renaming a note; type, `Enter` confirms, `Esc`
  cancels (a fresh note is kept even on `Esc`, just titled "New note")
- **ConfirmDelete** — `y`/`Enter` confirms, `n`/`Esc` cancels
- **Search** — live-as-you-type results with snippets, `Enter` jumps
- **Backlinks (focused)** — keyboard focus shifted onto the backlinks
  pane rather than a separate overlay
- **EditBody** — full-pane Markdown body editor, `Esc` saves and exits
- **Command** — the `:` prompt, replacing only the hint row
- **TagResults** — the full-pane list a `:tags` command opens
- **Toc** — the full-pane heading list `t` opens; `Enter` jumps, `x`
  extracts (see [[Table of contents and section extraction]])

Full-pane overlays (Search, EditBody, TagResults, Toc) replace the entire
screen; Command and ConfirmDelete replace only the hint row, leaving the
breadcrumb and the split-pane layout visible underneath; Backlinks-focused
just shifts border color and highlight within the existing pane.
