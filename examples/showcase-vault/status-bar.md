---
id: 8d6e7d31-ad1b-4654-87e1-aa6a4da20961
parent: bbf99ab4-d577-4227-8285-676cb40b0d47
order: 2
tags:
- interface
- status-bar
created: 2026-07-10T09:00:00Z
updated: 2026-07-13T18:00:00Z
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
  When there's room for it alongside the breadcrumb text and the
  marker, a third segment — the selected note's last-modified time
  (`modified: YYYY-MM-DD HH:MM`, UTC) — sits centered on the *whole*
  row, not just tacked on after the breadcrumb; on a narrow terminal
  it's dropped entirely rather than squeezed in.
- **Row 2 — hints**: a bold mode label, then keybinding hints tokenized
  on a `key: label` convention (bold key, dim colon, muted label).
  Normal mode's own hints are deliberately a short, curated subset —
  `j/k`, `a/o`, `e`, `d`, `u`, `/`, `?`, `q` — not the full keybinding
  set, which grew too long for any real terminal width over several
  versions; `?` opens a full-pane reference of everything else
  (dismissed by any keypress). With a read-only note selected, the
  three shown mutating hints that would just refuse (`a/o`, `e`, `d` —
  everything [[Guard every mutation against the wrong vault]] covers)
  dim down to the row's own separator style instead of sitting at full
  brightness for keys that won't do anything; same for an
  unmounted/archived vault's placeholder row. `u`/`^R` (undo/redo) are
  never dimmed — they aren't gated the same way, since the undo stack
  can never hold a foreign-vault action in the first place.

A prompt — delete confirmation, the quit-confirm notice, an error, the
[[Command palette]]'s input, or a status message — replaces row 2 only;
row 1's breadcrumb always stays visible above it.
