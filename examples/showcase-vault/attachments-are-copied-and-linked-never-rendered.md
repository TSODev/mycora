---
id: 0a23eb3b-c63f-47e5-b7d1-782e5c1b7d60
parent: e6fb9f85-2fbf-4f78-8689-d224e904e422
order: 15
tags:
- design-decision
- attachments
created: 2026-07-15T09:00:00Z
updated: 2026-07-15T09:00:00Z
---

# Attachments are copied and linked, never rendered

[[Attaching files to a note]] fits the "vault is just a directory of
files" philosophy (see [[Markdown as source of truth]]) cheaply: a
non-`.md` file sitting anywhere in the vault directory was already
inert to `load()` before this feature existed, and the Markdown
renderer already degraded an unhandled image link to its alt text
rather than crashing on it. Nothing about storage needed to change —
only a way to get a file *into* the vault and a link to it *into* the
cursor without doing both by hand.

The one real design fork was where "attach" could even be triggered
from: the `:` command palette only works in Normal/Command mode, and
"insert at the cursor" only means anything while actually editing a
body, where the cursor is a real `ratatui-textarea` position — a
`:attach <path>` command, the original shape floated for this, can't
reach that cursor at all. So it's `Ctrl+A` inside the body editor
instead, opening an inline prompt layered on top of `Mode::EditBody` the
same way the wikilink autocomplete popup (see
[[Cross-links and backlinks]]) already is, rather than a separate `:`
command or its own `Mode`.

Rendering the attached file inline (as an image, say) was explicitly
never the goal — the ask from the start was "even if we don't display
them," and that stayed true. This is about keeping a file linked
alongside a note, not turning Mycora into a media viewer.
