---
id: 3fc65d13-99f4-4a18-9874-d5ddf0ccc112
parent: bbf99ab4-d577-4227-8285-676cb40b0d47
order: 0
tags:
- interface
- layout
created: 2026-07-10T09:00:00Z
updated: 2026-07-11T01:00:00Z
---

# Layout

Three columns, plus the [[Status bar]] at the bottom. Every pane —
tree, body preview, backlinks, plus search/tag results elsewhere in the
app — scrolls to keep whatever's selected on screen; see
[[Every pane actually scrolls now]] for why that needed fixing at all.

- **Tree** (left, blue border) — the indented, collapsible note tree. If
  other vaults are mounted (see [[Multi-vault mounting]]), their notes
  appear stacked below it, dimmed and read-only but just as navigable —
  not roots-only.
- **Body preview** (middle, magenta border, with a little horizontal
  padding off the border) — the selected note's body, rendered as
  formatted Markdown (headings, bold/italic, code, lists, blockquotes,
  rules). Read-only; links and wikilinks render as plain text here, not
  as something clickable. `Ctrl+d`/`Ctrl+u` scroll it down/up, resetting
  to the top on every new selection. The padding is deliberately only
  here for now — continuous prose reads more cramped flush against a
  border than a short list row does, so this pane got it first; tree
  and backlinks stay flush, kept open to apply there too later.
- **Backlinks** (right) — notes linking to the selected note, live. No
  border color while idle; `b` moves keyboard focus into it (cyan border)
  — see [[Cross-links and backlinks]].

Column widths start at 40%/40%/20% and are resizable: `[`/`]` shrink/grow
the tree pane, `{`/`}` shrink/grow the backlinks pane, down to a 10%
floor per pane. Remembered across restarts, alongside the rest of
[[Session persistence]].

Search (`/`) and the body editor (`e`) still take over the whole screen as
full-pane overlays rather than living inside these columns; the backlinks
pane doesn't, since `b` shifts focus onto it in place instead.

Every color is a named terminal color, not RGB — see [[Theming]].
