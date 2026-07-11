---
id: 9d88bf5f-5b79-46f8-895b-a7c32ffe1261
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 8
tags:
- design-decision
- interface
created: 2026-07-11T01:00:00Z
updated: 2026-07-11T01:00:00Z
---

# Every pane actually scrolls now

Asked directly: had pane scrolling actually been verified to work? It
hadn't. Confirmed live against a 40-note generated test vault in a
15-row terminal: moving the selection down past the [[Layout]] tree
pane's visible rows changed the breadcrumb — selection genuinely
moved — but the pane kept rendering the exact same rows, the selected
one fully off-screen. A note with more sections than the body preview's
height got silently truncated, with no way to see the rest.

The root cause was the same across five panes (tree, backlinks, search
results, `:tags` results, `:tags list`): each built a plain list widget
and rendered it directly, never opting into the underlying toolkit's
stateful rendering path — the one that actually knows to keep a
selected row on screen. Without it, a list widget has no concept of
"the selection moved off-screen, scroll to follow it"; it just always
draws from the first item.

Before assuming a fix, checked the vendored widget-library source
directly rather than guessing at its scrolling behavior: it recomputes
the correct visible window from the current selection on every single
render call, even starting from a blank slate each time — so no new
persisted scroll state was needed for those five panes at all, just
switching them to the stateful rendering path. The body preview is
different: it's prose, not a list, so there's no "selected row" for the
toolkit to auto-follow. That one needed real new state — a scroll
offset, `Ctrl+d`/`Ctrl+u` (vim's own half-page-scroll keys) to move it,
and resetting to the top on every new selection so a freshly opened
note never starts mid-scroll from whatever the previous one left
behind.
