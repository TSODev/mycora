---
id: 8d6e7d31-ad1b-4654-87e1-aa6a4da20961
parent: bbf99ab4-d577-4227-8285-676cb40b0d47
order: 2
tags:
- interface
- status-bar
created: 2026-07-10T09:00:00Z
updated: 2026-07-12T14:00:00Z
---

# Status bar

A fixed two-line band at the bottom of the screen,
harmonized with the same convention used by Mycora's sibling terminal
tools (Terapi, jsoned): both rows share a dark indexed background
(`Color::Indexed(236)`), distinct from every other panel's default
background.

- **Row 1 — breadcrumb**: `vault › branch › note`, the selected note's
  ancestor titles from its tree root down to itself, with a dimmed,
  italic label right-aligned whenever the selection isn't an ordinary
  editable note — `READ-ONLY` for a read-only mounted vault,
  `UNMOUNTED`/`ARCHIVED` for one of those vaults' placeholder rows (see
  [[Multi-vault mounting]] and [[Unmounted vaults are visible too]]).
  Fixed-width so the breadcrumb's own text doesn't shift as the marker
  changes; blank but still painted with the row's background otherwise.
- **Row 2 — hints**: a bold mode label, then keybinding hints tokenized
  on a `key: label` convention (bold key, dim colon, muted label). In
  Normal mode with a read-only note selected, the seven hints for
  actions that would just refuse (`a/o`, `y`, `Tab/S-Tab`, `K/J`, `i`,
  `e`, `d` — everything [[Guard every mutation against the wrong vault]]
  covers) dim down to the row's own separator style instead of sitting
  at full brightness for keys that won't do anything. On an
  unmounted/archived vault's row, `h/l/space` (fold) dims too, since
  there's nothing loaded to expand at all. `u`/`^R` (undo/redo) are
  never dimmed — they aren't gated the same way, since the undo stack
  can never hold a foreign-vault action in the first place.

A prompt — delete confirmation, the quit-confirm notice, an error, the
[[Command palette]]'s input, or a status message — replaces row 2 only;
row 1's breadcrumb always stays visible above it.
