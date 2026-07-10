---
id: 8d6e7d31-ad1b-4654-87e1-aa6a4da20961
parent: bbf99ab4-d577-4227-8285-676cb40b0d47
order: 2
tags:
- interface
- status-bar
created: 2026-07-10T09:00:00Z
updated: 2026-07-10T23:00:00Z
---

# Status bar

A fixed two-line band at the bottom of the screen,
harmonized with the same convention used by Mycora's sibling terminal
tools (Terapi, jsoned): both rows share a dark indexed background
(`Color::Indexed(236)`), distinct from every other panel's default
background.

- **Row 1 — breadcrumb**: `vault › branch › note`, the selected note's
  ancestor titles from its tree root down to itself, with a dimmed,
  italic `READ-ONLY` label right-aligned whenever that selection is in a
  read-only mounted vault — see [[Multi-vault mounting]]. Fixed-width so
  the breadcrumb's own text doesn't shift as you move in and out of
  read-only vaults; blank but still painted with the row's background
  otherwise.
- **Row 2 — hints**: a bold mode label, then keybinding hints tokenized
  on a `key: label` convention (bold key, dim colon, muted label).

A prompt — delete confirmation, the quit-confirm notice, an error, the
[[Command palette]]'s input, or a status message — replaces row 2 only;
row 1's breadcrumb always stays visible above it.
