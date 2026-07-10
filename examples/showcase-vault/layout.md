---
id: 3fc65d13-99f4-4a18-9874-d5ddf0ccc112
parent: bbf99ab4-d577-4227-8285-676cb40b0d47
order: 0
tags:
- interface
- layout
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T09:00:00Z
---

# Layout

Three columns, plus the [[Status bar]] at the bottom:

- **Tree** (left, blue border) — the indented, collapsible note tree. If
  other vaults are mounted (see [[Multi-vault mounting]]), their root
  notes appear stacked below it, read-only.
- **Body preview** (middle, magenta border) — the selected note's body,
  rendered as formatted Markdown (headings, bold/italic, code, lists,
  blockquotes, rules). Read-only; links and wikilinks render as plain
  text here, not as something clickable.
- **Backlinks** (right) — notes linking to the selected note, live. No
  border color while idle; `b` moves keyboard focus into it (cyan border)
  — see [[Cross-links and backlinks]].

Column widths start at 40%/40%/20% and are resizable: `[`/`]` shrink/grow
the tree pane, `{`/`}` shrink/grow the backlinks pane, down to a 10%
floor per pane. Not persisted across restarts — a deliberate scope cut,
see [[Design decisions]].

Search (`/`) and the body editor (`e`) still take over the whole screen as
full-pane overlays rather than living inside these columns; the backlinks
pane doesn't, since `b` shifts focus onto it in place instead.

Every color is a named terminal color, not RGB — see [[Theming]].
